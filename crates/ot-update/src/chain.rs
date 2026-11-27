//! The whole chain, end to end, against a local server (FEAT-0078): a signed
//! manifest is fetched and verified, the exe is downloaded and held to the
//! manifest's size and digest, and it is staged beside a stand-in running program.
//!
//! The manifests are signed with a throwaway key made at run time (so no private
//! key is ever committed) and verified against that key's public half, exactly as
//! production does with the embedded one. Each attack the spec names has its own
//! test: a tampered manifest, a substituted download, an oversize one, a replayed
//! old release, a copy too old to update in place, a read-only folder.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::path::Path;

use minisign::KeyPair;
use semver::Version;
use sha2::{Digest, Sha256};

use ot_net::Cancel;

use crate::check::{Client, PrepareError};
use crate::download::{DownloadError, Downloader, hex};
use crate::release::{Asset, Release};
use crate::stage::{StageError, stage, staged_path};
use crate::verify::VerifyError;

/// A server for the whole chain: answers each path with its body, for as many
/// connections as come, until the test ends.
fn serve(routes: HashMap<String, Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut raw = Vec::new();
            let mut chunk = [0u8; 1024];
            while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => raw.extend_from_slice(&chunk[..n]),
                }
            }
            let request = String::from_utf8_lossy(&raw).into_owned();
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
            let (status, body) = match routes.get(&path) {
                Some(body) => ("200 OK", body.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let mut answer = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            answer.extend_from_slice(&body);
            let _ = stream.write_all(&answer);
            let _ = stream.flush();
            let _ = stream.read(&mut chunk);
        }
    });
    base
}

fn v(text: &str) -> Version {
    Version::parse(text).expect("semver")
}

/// Everything a release needs, signed by a fresh key.
struct Fixture {
    base: String,
    release: Release,
    public_key: String,
    exe: Vec<u8>,
}

/// A release of `version` whose exe is `exe`, with a manifest saying
/// `manifest_size` and `manifest_digest` (the truth, unless a test lies), served
/// over HTTP. `min_supported` is what the manifest allows to update in place.
fn release_of(
    version: &str,
    exe: &[u8],
    manifest_size: Option<u64>,
    manifest_digest: Option<String>,
    min_supported: &str,
    tamper_manifest_after_signing: bool,
) -> Fixture {
    let pair = KeyPair::generate_unencrypted_keypair().expect("a key");
    let public_key = pair.pk.to_box().expect("a box").to_string();
    // The port is not known until the listener exists, so the manifest is built
    // from a placeholder and the server started from its final bytes.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    drop(listener);
    let _ = base;

    let digest = hex(&Sha256::digest(exe));
    let manifest = |base: &str| {
        format!(
            r#"{{"schema":1,"product":"OmniToaster","version":"{version}","channel":"stable","tag":"v{version}","notes_url":"{base}/notes","min_supported_version":"{min_supported}","data_schema":1,"assets":{{"windows-x64":{{"name":"OmniToaster.exe","url":"{base}/OmniToaster.exe","size":{size},"sha256":"{digest}"}}}}}}"#,
            size = manifest_size.unwrap_or(exe.len() as u64),
            digest = manifest_digest.clone().unwrap_or(digest.clone()),
        )
    };
    // Two passes: pick a port by serving, then build routes that name it.
    let probe = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = probe.local_addr().expect("addr").port();
    drop(probe);
    let base = format!("http://127.0.0.1:{port}");
    let manifest_bytes = manifest(&base).into_bytes();
    let signature = minisign::sign(
        Some(&pair.pk),
        &pair.sk,
        Cursor::new(&manifest_bytes),
        Some("test release"),
        None,
    )
    .expect("signs")
    .to_string();
    let served_manifest = if tamper_manifest_after_signing {
        let mut bytes = manifest_bytes.clone();
        let at = bytes
            .windows(6)
            .position(|w| w == b"\"size\"")
            .expect("a size");
        bytes[at + 8] = b'9';
        bytes
    } else {
        manifest_bytes
    };
    let mut routes = HashMap::new();
    routes.insert("/manifest.json".to_owned(), served_manifest);
    routes.insert("/manifest.json.minisig".to_owned(), signature.into_bytes());
    routes.insert("/OmniToaster.exe".to_owned(), exe.to_vec());
    // Bind the real server on the chosen port.
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("the port is free");
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut raw = Vec::new();
            let mut chunk = [0u8; 1024];
            while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => raw.extend_from_slice(&chunk[..n]),
                }
            }
            let request = String::from_utf8_lossy(&raw).into_owned();
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
            let (status, body) = match routes.get(&path) {
                Some(body) => ("200 OK", body.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let mut answer = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            answer.extend_from_slice(&body);
            let _ = stream.write_all(&answer);
            let _ = stream.flush();
            let _ = stream.read(&mut chunk);
        }
    });
    let asset = |name: &str| Asset {
        name: name.to_owned(),
        url: format!("{base}/{name}"),
        size: 0,
    };
    Fixture {
        release: Release {
            version: v(version),
            tag: format!("v{version}"),
            page_url: format!("{base}/page"),
            assets: vec![
                asset("manifest.json"),
                asset("manifest.json.minisig"),
                asset("OmniToaster.exe"),
            ],
        },
        base,
        public_key,
        exe: exe.to_vec(),
    }
}

fn honest(version: &str) -> Fixture {
    release_of(
        version,
        b"MZ pretend this is OmniToaster.exe, a few bytes of it",
        None,
        None,
        "0.1.0",
        false,
    )
}

#[test]
fn the_whole_chain_installs_nothing_but_stages_a_verified_exe_beside_the_running_one() {
    let fixture = honest("0.19.0");
    let client = Client::local("http://127.0.0.1:9/unused");
    let manifest = client
        .verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &Cancel::new(),
        )
        .expect("a genuine release verifies");
    assert_eq!(manifest.version, v("0.19.0"));

    let folder = tempfile::tempdir().expect("a scratch folder");
    let cache = folder.path().join("OmniToasterData/cache/update");
    let mut seen = Vec::new();
    let downloaded = Downloader::local()
        .download(
            &manifest.exe,
            &manifest.version,
            &cache,
            &Cancel::new(),
            &mut |done, total| {
                seen.push((done, total));
            },
        )
        .expect("downloads and verifies");
    assert_eq!(std::fs::read(&downloaded).expect("reads"), fixture.exe);
    assert_eq!(downloaded, cache.join("OmniToaster-0.19.0.exe"));
    assert_eq!(
        seen.last().copied(),
        Some((fixture.exe.len() as u64, fixture.exe.len() as u64)),
        "progress reached the end"
    );

    let running = folder.path().join("OmniToaster.exe");
    std::fs::write(&running, b"the old program").expect("writes");
    let staged = stage(&downloaded, &running, &manifest.exe.sha256).expect("stages");
    assert_eq!(staged, folder.path().join("OmniToaster.exe.new"));
    assert_eq!(std::fs::read(&staged).expect("reads"), fixture.exe);
    assert_eq!(
        std::fs::read(&running).expect("reads"),
        b"the old program",
        "the running exe is untouched"
    );
    // Only the cache and the staged file were written.
    let mut names: Vec<_> = std::fs::read_dir(folder.path())
        .expect("lists")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["OmniToaster.exe", "OmniToaster.exe.new", "OmniToasterData"]
    );
}

#[test]
fn a_manifest_altered_after_signing_is_rejected_and_no_download_follows() {
    let fixture = release_of("0.19.0", b"payload", None, None, "0.1.0", true);
    let error = Client::local("http://127.0.0.1:9/unused")
        .verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &Cancel::new(),
        )
        .expect_err("refused");
    assert_eq!(error, PrepareError::Verify(VerifyError::Rejected));
}

#[test]
fn a_manifest_signed_by_someone_else_is_rejected() {
    let fixture = honest("0.19.0");
    let other = KeyPair::generate_unencrypted_keypair().expect("a key");
    let their_key = other.pk.to_box().expect("a box").to_string();
    let error = Client::local("http://127.0.0.1:9/unused")
        .verified_manifest(&fixture.release, &v("0.18.0"), &their_key, &Cancel::new())
        .expect_err("refused");
    assert_eq!(error, PrepareError::Verify(VerifyError::Rejected));
}

#[test]
fn an_old_signed_release_replayed_as_the_latest_is_not_newer() {
    let fixture = honest("0.17.0");
    let client = Client::local("http://127.0.0.1:9/unused");
    assert_eq!(
        client.verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &Cancel::new()
        ),
        Err(PrepareError::NotNewer(v("0.17.0")))
    );
    assert_eq!(
        client.verified_manifest(
            &fixture.release,
            &v("0.17.0"),
            &fixture.public_key,
            &Cancel::new()
        ),
        Err(PrepareError::NotNewer(v("0.17.0"))),
        "the same version is not an update either"
    );
}

#[test]
fn a_manifest_for_another_version_than_the_release_says_is_refused() {
    let mut fixture = honest("0.19.0");
    fixture.release.version = v("0.20.0");
    assert!(matches!(
        Client::local("http://127.0.0.1:9/unused").verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &Cancel::new()
        ),
        Err(PrepareError::VersionMismatch { .. })
    ));
}

#[test]
fn a_copy_older_than_the_release_updates_in_place_from_is_sent_to_the_zip() {
    let fixture = release_of("0.19.0", b"payload", None, None, "0.18.5", false);
    assert_eq!(
        Client::local("http://127.0.0.1:9/unused").verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &Cancel::new()
        ),
        Err(PrepareError::ManualUpdateNeeded(v("0.18.5")))
    );
}

#[test]
fn a_release_missing_its_signature_or_manifest_is_refused_before_any_request() {
    let mut fixture = honest("0.19.0");
    fixture
        .release
        .assets
        .retain(|a| a.name != "manifest.json.minisig");
    assert_eq!(
        Client::local("http://127.0.0.1:9/unused").verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &Cancel::new()
        ),
        Err(PrepareError::MissingAsset("manifest.json.minisig"))
    );
}

#[test]
fn a_cancel_before_the_manifest_is_fetched_stops_the_check() {
    let fixture = honest("0.19.0");
    let cancel = Cancel::new();
    cancel.cancel();
    assert_eq!(
        Client::local("http://127.0.0.1:9/unused").verified_manifest(
            &fixture.release,
            &v("0.18.0"),
            &fixture.public_key,
            &cancel
        ),
        Err(PrepareError::Cancelled)
    );
}

/// What a download does when the server's bytes are not the signed ones.
fn download_of(
    fixture: &Fixture,
    entry_size: Option<u64>,
    entry_digest: Option<String>,
) -> (Result<std::path::PathBuf, DownloadError>, tempfile::TempDir) {
    let mut entry = crate::manifest::AssetEntry {
        name: "OmniToaster.exe".into(),
        url: format!("{}/OmniToaster.exe", fixture.base),
        size: fixture.exe.len() as u64,
        sha256: hex(&Sha256::digest(&fixture.exe)),
    };
    if let Some(size) = entry_size {
        entry.size = size;
    }
    if let Some(digest) = entry_digest {
        entry.sha256 = digest;
    }
    let folder = tempfile::tempdir().expect("a scratch folder");
    let result = Downloader::local().download(
        &entry,
        &v("0.19.0"),
        &folder.path().join("cache"),
        &Cancel::new(),
        &mut |_, _| {},
    );
    (result, folder)
}

fn cache_is_empty(folder: &Path) -> bool {
    std::fs::read_dir(folder.join("cache"))
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true)
}

#[test]
fn a_substituted_download_is_rejected_and_deleted() {
    let fixture = honest("0.19.0");
    // The manifest says the file hashes to something else: what a swapped binary looks like.
    let (result, folder) = download_of(&fixture, None, Some("0".repeat(64)));
    assert!(
        matches!(result, Err(DownloadError::WrongDigest)),
        "{result:?}"
    );
    assert!(
        cache_is_empty(folder.path()),
        "nothing wrong is left on disk"
    );
}

#[test]
fn a_download_larger_than_the_manifest_says_is_abandoned_and_deleted() {
    let fixture = honest("0.19.0");
    let (result, folder) = download_of(&fixture, Some(10), None);
    assert!(
        matches!(result, Err(DownloadError::TooLarge(_))),
        "{result:?}"
    );
    assert!(cache_is_empty(folder.path()));
}

#[test]
fn a_download_shorter_than_the_manifest_says_is_rejected_and_deleted() {
    let fixture = honest("0.19.0");
    let (result, folder) = download_of(&fixture, Some(fixture.exe.len() as u64 + 100), None);
    assert!(
        matches!(result, Err(DownloadError::WrongSize { .. })),
        "{result:?}"
    );
    assert!(cache_is_empty(folder.path()));
}

#[test]
fn a_cancel_during_the_transfer_stops_it_and_leaves_nothing() {
    let exe = vec![7u8; 1 << 20];
    let fixture = release_of("0.19.0", &exe, None, None, "0.1.0", false);
    let entry = crate::manifest::AssetEntry {
        name: "OmniToaster.exe".into(),
        url: format!("{}/OmniToaster.exe", fixture.base),
        size: exe.len() as u64,
        sha256: hex(&Sha256::digest(&exe)),
    };
    let cancel = Cancel::new();
    let folder = tempfile::tempdir().expect("a scratch folder");
    let result = Downloader::local().download(
        &entry,
        &v("0.19.0"),
        &folder.path().join("cache"),
        &cancel,
        // The first chunk arrives, and the user cancels.
        &mut |_, _| cancel.cancel(),
    );
    assert!(
        matches!(result, Err(DownloadError::Cancelled)),
        "{result:?}"
    );
    assert!(
        cache_is_empty(folder.path()),
        "the half-downloaded file is gone"
    );
}

#[test]
fn a_missing_file_and_an_unreachable_server_are_named_not_panics() {
    let fixture = honest("0.19.0");
    let mut entry = crate::manifest::AssetEntry {
        name: "OmniToaster.exe".into(),
        url: format!("{}/nothing-here", fixture.base),
        size: 5,
        sha256: "0".repeat(64),
    };
    let folder = tempfile::tempdir().expect("a scratch folder");
    let cache = folder.path().join("cache");
    let result =
        Downloader::local().download(&entry, &v("0.19.0"), &cache, &Cancel::new(), &mut |_, _| {});
    assert!(
        matches!(result, Err(DownloadError::Http(404))),
        "{result:?}"
    );
    entry.url = "http://127.0.0.1:9/x".into();
    let result =
        Downloader::local().download(&entry, &v("0.19.0"), &cache, &Cancel::new(), &mut |_, _| {});
    assert!(
        matches!(result, Err(DownloadError::Network(_))),
        "{result:?}"
    );
}

#[test]
fn staging_replaces_an_earlier_staged_file_and_a_corrupt_copy_is_refused() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let running = folder.path().join("OmniToaster.exe");
    std::fs::write(&running, b"old").expect("writes");
    let good = folder.path().join("good.exe");
    std::fs::write(&good, b"new bytes").expect("writes");
    let digest = hex(&Sha256::digest(b"new bytes"));
    std::fs::write(staged_path(&running).expect("path"), b"an earlier stage").expect("writes");
    let staged = stage(&good, &running, &digest).expect("stages");
    assert_eq!(
        std::fs::read(&staged).expect("reads"),
        b"new bytes",
        "replaced"
    );

    // A copy whose digest is not the verified one is not kept.
    let error = stage(&good, &running, &"1".repeat(64)).expect_err("refused");
    assert!(matches!(error, StageError::CopyMismatch), "{error:?}");
    assert!(
        !folder.path().join("OmniToaster.new.part").exists()
            && !folder.path().join("OmniToaster.exe.new.part").exists(),
        "no part file is left"
    );
}

#[test]
fn a_folder_that_cannot_be_written_says_so_instead_of_crashing() {
    // The running program's folder does not exist, so nothing can be written there:
    // the same failure a read-only Program Files gives, and what the pane turns into
    // "use the zip".
    let folder = tempfile::tempdir().expect("a scratch folder");
    let good = folder.path().join("good.exe");
    std::fs::write(&good, b"new bytes").expect("writes");
    let running = folder.path().join("no-such-folder").join("OmniToaster.exe");
    let error = stage(&good, &running, &hex(&Sha256::digest(b"new bytes"))).expect_err("refused");
    assert!(matches!(error, StageError::NotWritable { .. }), "{error:?}");
}

#[test]
fn the_staged_path_is_the_running_exes_name_with_dot_new() {
    assert_eq!(
        staged_path(Path::new(r"D:\Apps\OmniToaster\OmniToaster.exe")).expect("path"),
        Path::new(r"D:\Apps\OmniToaster\OmniToaster.exe.new")
    );
}

// Keep the helper server used by the two-pass builder honest: it serves what it is given.
#[test]
fn the_test_server_answers_known_paths_and_404s_the_rest() {
    let mut routes = HashMap::new();
    routes.insert("/a".to_owned(), b"hello".to_vec());
    let base = serve(routes);
    let agent = ot_net::agent(false);
    let mut ok = agent.get(&format!("{base}/a")).call().expect("answers");
    assert_eq!(ok.body_mut().read_to_vec().expect("body"), b"hello");
    let missing = agent.get(&format!("{base}/b")).call().expect("answers");
    assert_eq!(missing.status().as_u16(), 404);
}

/// The real thing, against both forges: find the release, verify its signature with
/// the key embedded in this crate, download the real `OmniToaster.exe` (about 16 MB
/// each), hold it to the signed digest, and stage it beside a stand-in. Nothing is
/// run or installed. Not run by default — it downloads two programs:
/// `cargo test -p ot-update -- --ignored a_live_release`.
#[test]
#[ignore = "downloads OmniToaster.exe (about 16 MB) from git.zacx.net and from github.com"]
fn a_live_release_verifies_downloads_and_stages_from_both_forges() {
    use crate::check::Outcome;
    use crate::source::Repo;
    use crate::verify::PUBLIC_KEY;

    for repo in [
        "https://git.zacx.net/owner/omni-toaster",
        "https://github.com/mzaki-raup/omni-toaster",
    ] {
        let running = v("0.1.0");
        let client = Client::for_repo(&Repo::parse(repo).expect("parses"));
        let Outcome::Available(release) = client
            .check(&running, &Cancel::new())
            .unwrap_or_else(|e| panic!("{repo}: {e}"))
        else {
            panic!("{repo}: no release offered over 0.1.0");
        };
        let manifest = client
            .verified_manifest(&release, &running, PUBLIC_KEY, &Cancel::new())
            .unwrap_or_else(|e| panic!("{repo}: the real signature did not verify: {e}"));
        assert!(
            manifest
                .exe
                .url
                .contains(&Repo::parse(repo).expect("parses").host),
            "{repo}: {}",
            manifest.exe.url
        );

        let folder = tempfile::tempdir().expect("a scratch folder");
        let downloaded = Downloader::new()
            .download(
                &manifest.exe,
                &manifest.version,
                &folder.path().join("cache"),
                &Cancel::new(),
                &mut |_, _| {},
            )
            .unwrap_or_else(|e| panic!("{repo}: {e}"));
        let running_exe = folder.path().join("OmniToaster.exe");
        std::fs::write(&running_exe, b"stand-in").expect("writes");
        let staged = stage(&downloaded, &running_exe, &manifest.exe.sha256).expect("stages");
        assert_eq!(
            std::fs::metadata(&staged).expect("meta").len(),
            manifest.exe.size,
            "{repo}"
        );
    }
}
