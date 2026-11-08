<#
.SYNOPSIS
  Regenerates every app icon size from the one master image.

.DESCRIPTION
  The master is assets/icon/omnitoaster-512.png. Everything else in
  assets/icon/ is derived and must never be edited by hand - run this instead,
  so the sizes cannot drift apart from each other or from the master
  (.claude/rules/build-and-run.md).

  Produces:
    omnitoaster-{256,128,64,48,32,24,16}.png   tray, README, zip
    omnitoaster.ico                            the executable icon

  The .ico carries every size from 16 to 256. Windows picks per context:
  16 in the title bar and the tray, 32 in the taskbar, 48 in Explorer's
  medium view, 256 for extra-large icons and the Alt-Tab switcher.

  Needs ImageMagick 7 (`magick`). Lanczos is the resampling filter: the
  icon has thin elements - the bell's rays - and a box filter loses them
  entirely below 48 px.

  Sizes at or below 32 px also get an unsharp pass. Downscaling this
  artwork that far softens it into a coloured blob in which the card
  stack and the bell are no longer distinguishable - and 16 px is the
  tray and title-bar size, so it is the one users look at most. The
  sharpen brings back the card edges, the bell silhouette and the rays.
  48 px and above are left alone: they are already crisp and sharpening
  them only looks crunchy.

.EXAMPLE
  pwsh -File scripts/make-icons.ps1
#>
param(
  [string]$Master = "assets/icon/omnitoaster-512.png",
  [string]$OutDir = "assets/icon",
  [switch]$Check
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command magick -ErrorAction SilentlyContinue)) {
  throw "ImageMagick 7 is not installed: https://imagemagick.org (the `magick` command must be on PATH)."
}
if (-not (Test-Path $Master)) {
  throw "The master image $Master is missing. It is the one file here that is not generated."
}

# The master must be square and large enough that every size below is a
# downscale; upscaling an icon produces a blurred mess that looks like a bug.
$geometry = (magick identify -format "%w %h" $Master) -split ' '
[int]$w = $geometry[0]; [int]$h = $geometry[1]
if ($w -ne $h) { throw "The master is ${w}x${h}; an icon master must be square." }
if ($w -lt 512) { throw "The master is ${w}px; 512 or larger is needed so no size is upscaled." }

$sizes = @(256, 128, 64, 48, 32, 24, 16)
$made = @()

# At or below this, the downscale needs help to stay legible.
$SharpenAtOrBelow = 32

foreach ($size in $sizes) {
  $out = Join-Path $OutDir "omnitoaster-$size.png"
  if ($Check) { $made += $out; continue }

  if ($size -le $SharpenAtOrBelow) {
    magick $Master -filter Lanczos -resize "${size}x${size}" `
      -unsharp 0x0.6+0.9+0.02 -strip -define png:compression-level=9 $out
  } else {
    magick $Master -filter Lanczos -resize "${size}x${size}" `
      -strip -define png:compression-level=9 $out
  }
  if ($LASTEXITCODE -ne 0) { throw "magick failed producing $out" }
  $made += $out
}

# One .ico holding every size. ImageMagick stores the 256 entry PNG-compressed,
# which is what keeps the file small enough to embed without complaint.
$ico = Join-Path $OutDir "omnitoaster.ico"
if (-not $Check) {
  $inputs = $sizes | ForEach-Object { Join-Path $OutDir "omnitoaster-$_.png" }
  magick @inputs $ico
  if ($LASTEXITCODE -ne 0) { throw "magick failed producing $ico" }
}
$made += $ico

if ($Check) {
  $missing = @($made | Where-Object { -not (Test-Path $_) })
  if ($missing.Count) { throw "Missing generated icons: $($missing -join ', '). Run scripts/make-icons.ps1." }
  Write-Host "All $($made.Count) generated icons are present."
  exit 0
}

Write-Host "Regenerated from $Master :"
foreach ($file in $made) {
  $item = Get-Item $file
  Write-Host ("  {0,-34} {1,8:N0} bytes" -f $item.Name, $item.Length)
}
exit 0
