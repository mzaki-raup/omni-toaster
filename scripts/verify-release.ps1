<#
.SYNOPSIS
  Verifies a published OmniToaster release exactly as the in-app updater would.

.DESCRIPTION
  Reads the release for -Tag from the forge API, then:
    1. the release is not a draft and has a non-empty body;
    2. every contracted asset is attached;
    3. manifest.json downloads (anonymously, like the app) and its minisign
       signature verifies against keys/minisign.pub;
    4. the manifest's version, tag and channel match the release;
    5. every asset listed in the manifest downloads with the stated size and
       sha256, and SHA256SUMS.txt agrees;
    6. the exe was built for the forge it is published on: the update source it
       carries (what the in-app updater reads) is this forge's, and
       the manifest's download and notes addresses are too. GitHub and the
       Forgejo each build their own release, so their exes and manifests
       differ by design and each one points at its own forge;
    7. the Windows executable prints the released version with --version and
       passes --self-test against a temporary data folder.

  The release workflow runs this as its last step; a failure fails the job so a
  broken release is never left looking fine. Run it locally against any tag to
  check what users will get.
#>
param(
  [Parameter(Mandatory = $true)][string]$Tag,
  # owner/repo on the forge; defaults to the origin remote.
  [string]$Repository = "",
  # Forgejo instance, or https://github.com for the public mirror. Forgejo
  # cloned GitHub's releases API, so only the base URL differs and everything
  # below works unchanged against either.
  [string]$Server = "https://git.zacx.net",
  [string]$PublicKey = "keys/minisign.pub",
  # Used only for the release metadata request, never for an asset download:
  # github.com limits anonymous calls to 60/hour per IP and a shared Actions
  # runner can exhaust that before this step runs. Assets are still fetched
  # anonymously, exactly as the app fetches them.
  [string]$Token = $(if ($env:GITHUB_TOKEN) { $env:GITHUB_TOKEN } elseif ($env:GH_TOKEN) { $env:GH_TOKEN } else { "" }),
  [switch]$NoSelfTest,
  # For a release published before the exe carried its update source (before
  # v0.19.0): skips step 6's check of the exe's own bytes.
  [switch]$SkipBakedSource
)

$ErrorActionPreference = "Stop"

# The address this forge's releases live at: what build-and-package.ps1 was given
# as -RepoUrl, so it is what the exe carries and what the manifest's links start with.
function Get-RepoUrl([string]$Server, [string]$Repository) { return ($Server.TrimEnd('/') + "/" + $Repository) }

# What the exe carries when it was built for `RepoUrl`: ot-update's build script
# writes it (crates/ot-update/build.rs), and only the repository it was built for is in it.
function Get-SourceMarker([string]$RepoUrl) { return "[omnitoaster-update-source=$RepoUrl]" }

# Whether a file's bytes contain the address. A compiled string is stored as plain
# ASCII, so a byte search is enough; the whole exe is read, which is a few megabytes.
function Test-ContainsText([string]$Path, [string]$Text) {
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $needle = [System.Text.Encoding]::ASCII.GetBytes($Text)
  if ($needle.Length -eq 0 -or $bytes.Length -lt $needle.Length) { return $false }
  $first = $needle[0]
  $last = $bytes.Length - $needle.Length
  for ($i = [Array]::IndexOf($bytes, $first, 0); $i -ge 0 -and $i -le $last; $i = [Array]::IndexOf($bytes, $first, $i + 1)) {
    $same = $true
    for ($k = 1; $k -lt $needle.Length; $k++) { if ($bytes[$i + $k] -ne $needle[$k]) { $same = $false; break } }
    if ($same) { return $true }
  }
  return $false
}

# Dot-sourced by a test: stop after the definitions above.
if ($env:OT_VERIFY_FUNCTIONS_ONLY) { return }

if (-not $Repository) {
  $origin = git remote get-url origin
  if ($origin -match '[:/]([^/]+/[^/]+?)(\.git)?$') { $Repository = $Matches[1] } else { throw "Cannot derive owner/repo from '$origin'; pass -Repository." }
}
if (-not (Test-Path $PublicKey)) { throw "Public key $PublicKey is missing." }
if (-not (Get-Command rsign -ErrorAction SilentlyContinue)) { throw "rsign (rsign2) is not installed: cargo install rsign2 --locked" }

# GitHub puts the API on a separate host; Forgejo serves it under /api/v1 on
# the instance itself. Anonymous github.com calls are rate-limited to 60/hour
# per IP, which is ample for one release but will bite a tight retry loop.
$api = if ($Server -match '^https://(api\.)?github\.com/?$') {
  "https://api.github.com/repos/$Repository"
} else {
  "$Server/api/v1/repos/$Repository"
}
$ua  = @{ "User-Agent" = "OmniToaster-verify-release" }
$metaHeaders = @{ "User-Agent" = "OmniToaster-verify-release" }
if ($Token) { $metaHeaders["Authorization"] = "Bearer $Token" }

# Retry the metadata read: github.com answers 403/429 when the shared runner
# IP has used its anonymous hour, and a transient 5xx should not fail a
# release that is otherwise fine.
$release = $null
for ($attempt = 1; ; $attempt++) {
  try {
    $release = Invoke-RestMethod -Method Get -Headers $metaHeaders -Uri "$api/releases/tags/$([uri]::EscapeDataString($Tag))" -TimeoutSec 60
    break
  } catch {
    $code = 0
    try { $code = [int]$_.Exception.Response.StatusCode } catch { }
    if ($attempt -ge 3 -or ($code -ne 403 -and $code -ne 429 -and $code -lt 500)) { throw }
    $wait = 10 * $attempt
    Write-Host "Release metadata read returned $code (attempt $attempt); retrying in $wait s."
    Start-Sleep -Seconds $wait
  }
}

if ($release.draft) { throw "Release $Tag is still a draft." }
if ([string]::IsNullOrWhiteSpace($release.body)) { throw "Release $Tag has an empty body." }
$version = $Tag -replace '^v', ''
$assets = @{}
foreach ($a in $release.assets) { $assets[$a.name] = $a }

$required = @("OmniToaster.exe", "OmniToaster-$version-windows-x64.zip", "manifest.json", "manifest.json.minisig", "SHA256SUMS.txt", "CHANGELOG-$version.md")
$missing = @($required | Where-Object { -not $assets.ContainsKey($_) })
if ($missing.Count) { throw "Release $Tag lacks required assets: $($missing -join ', ')" }

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("ot-verify-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  function Fetch([string]$name) {
    $path = Join-Path $work $name
    for ($attempt = 1; ; $attempt++) {
      try {
        Invoke-WebRequest -Headers $ua -Uri $assets[$name].browser_download_url -OutFile $path -TimeoutSec 600
        return $path
      } catch {
        # A half-written file would fail the hash anyway; start clean.
        Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
        if ($attempt -ge 4) {
          throw "Downloading $name failed after $attempt attempts: $($_.Exception.Message)"
        }
        $wait = 3 * [math]::Pow(2, $attempt - 1)
        Write-Host "Downloading $name failed (attempt $attempt): $($_.Exception.Message). Retrying in $wait s."
        Start-Sleep -Seconds $wait
      }
    }
  }

  $manifestPath = Fetch "manifest.json"
  $sigPath      = Fetch "manifest.json.minisig"
  rsign verify -p $PublicKey -x $sigPath $manifestPath
  if ($LASTEXITCODE -ne 0) { throw "manifest.json signature does NOT verify against $PublicKey." }
  Write-Host "manifest.json signature OK"

  $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
  if ($manifest.version -ne $version) { throw "manifest.version '$($manifest.version)' != '$version'." }
  if ($manifest.tag -ne $Tag)         { throw "manifest.tag '$($manifest.tag)' != '$Tag'." }
  $expectedChannel = if ($release.prerelease) { "beta" } else { "stable" }
  if ($manifest.channel -ne $expectedChannel) { throw "manifest.channel '$($manifest.channel)' but the forge marks the release as $expectedChannel." }

  $sumsPath = Fetch "SHA256SUMS.txt"
  $sums = @{}
  foreach ($line in Get-Content $sumsPath) { if ($line -match '^([0-9a-f]{64})\s+\*?(.+)$') { $sums[$Matches[2]] = $Matches[1] } }

  $repoUrl = Get-RepoUrl $Server $Repository
  if (-not ([string]$manifest.notes_url).StartsWith("$repoUrl/")) { throw "manifest.notes_url '$($manifest.notes_url)' is not on $repoUrl." }

  $exePath = $null
  foreach ($key in $manifest.assets.PSObject.Properties.Name) {
    $entry = $manifest.assets.$key
    if (-not $assets.ContainsKey($entry.name)) { throw "Manifest asset '$($entry.name)' is not attached to the release." }
    if (-not ([string]$entry.url).StartsWith("$repoUrl/")) { throw "Manifest asset '$($entry.name)' points at '$($entry.url)', not at $repoUrl." }
    $path = Fetch $entry.name
    $size = (Get-Item $path).Length
    $hash = (Get-FileHash $path -Algorithm SHA256).Hash.ToLower()
    if ($size -ne $entry.size)     { throw "$($entry.name): size $size != manifest $($entry.size)." }
    if ($hash -ne $entry.sha256)   { throw "$($entry.name): sha256 mismatch against manifest." }
    if ($sums[$entry.name] -ne $hash) { throw "$($entry.name): sha256 mismatch against SHA256SUMS.txt." }
    Write-Host "$($entry.name): size and sha256 OK"
    if ($key -eq "windows-x64") { $exePath = $path }
  }

  if (-not $SkipBakedSource -and $exePath) {
    # The marker, not the bare address: the default repository is in every build as
    # a constant, so a GitHub exe contains the Forgejo's address too.
    if (-not (Test-ContainsText $exePath (Get-SourceMarker $repoUrl))) {
      throw "OmniToaster.exe does not carry the update source $repoUrl, so it would look for its updates on another forge. It was built for the wrong one."
    }
    Write-Host "OmniToaster.exe carries its update source, $repoUrl"
  }

  if (-not $NoSelfTest -and $exePath -and $IsWindows) {
    # GUI-subsystem exe: use Start-Process -Wait to get real exit codes.
    $versionOut = Join-Path $work "version.txt"
    $p = Start-Process -FilePath $exePath -ArgumentList @('--version') -Wait -PassThru -NoNewWindow -RedirectStandardOutput $versionOut
    $reported = (Get-Content $versionOut -Raw -ErrorAction SilentlyContinue | Out-String).Trim()
    if ($p.ExitCode -ne 0 -or $reported -notmatch [regex]::Escape($version)) { throw "Downloaded exe reports '$reported', expected $version." }
    $data = Join-Path $work "data"
    $p = Start-Process -FilePath $exePath -ArgumentList @('--self-test', '--data-dir', $data) -Wait -PassThru -NoNewWindow
    if ($p.ExitCode -ne 0) { throw "Downloaded exe failed --self-test with exit code $($p.ExitCode)." }
    Write-Host "OmniToaster.exe --version and --self-test OK"
  }

  Write-Host "Release $Tag verified: $($required.Count) assets, signed manifest, hashes and self-test all OK."
} finally {
  Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
exit 0
