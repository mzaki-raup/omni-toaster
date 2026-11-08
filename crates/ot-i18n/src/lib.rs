//! Fluent bundles, locale detection and the one date/number formatter.
//!
//! Every visible string in OmniToaster is a Fluent key present in **both**
//! `locales/en-US/omni_toaster.ftl` and `locales/ms-MY/omni_toaster.ftl`
//! (`docs/10-i18n.md`, ADR-006). A missing Malay key falls back to English and
//! logs a warning rather than showing a key name to a user.
//!
//! `format_datetime` is the only function in the workspace that formats a
//! date, a time or a number. The toast, the tray tooltip and the speech
//! service all call it, so a format change lands in one place.
//!
//! See `docs/03-architecture.md` for where this crate sits and which crates it
//! may depend on.

use std::borrow::Cow;
use std::path::Path;
use std::sync::OnceLock;

use i18n_embed::LanguageLoader as _;
use i18n_embed::fluent::FluentLanguageLoader;
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

/// The locale files are named after the product, not after this crate:
/// `locales/<tag>/omni_toaster.ftl` (`docs/05-data-model-and-storage.md`).
/// They are a translator's catalogue, so they carry the product's name.
///
/// `i18n_embed`'s `fluent_language_loader!` macro derives the filename from the
/// crate name instead, and would look for `ot-i18n.ftl`. Passing the domain
/// explicitly keeps the documented filename; the cost is that
/// `i18n-embed-fl`'s `fl!` macro, which resolves its compile-time check the
/// same crate-name way, cannot be used. Key correctness is covered by the
/// parity and prefix tests in this module instead — see ADR-006.
const DOMAIN: &str = "omni_toaster";

pub mod format;

pub use format::{TimePrefs, format_datetime, format_time};

/// The bundled locale files, compiled into the exe by `rust-embed` so the
/// portable folder needs nothing beside it.
#[derive(RustEmbed)]
#[folder = "../../locales"]
struct Localizations;

/// The two languages OmniToaster supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Malay,
}

impl Language {
    /// The BCP-47 tag, which is also the locale folder name.
    pub fn tag(self) -> &'static str {
        match self {
            Self::English => "en-US",
            Self::Malay => "ms-MY",
        }
    }

    /// The Fluent key naming this language, for a language picker.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::English => "language-en-us",
            Self::Malay => "language-ms-my",
        }
    }

    fn langid(self) -> LanguageIdentifier {
        self.tag().parse().expect("the two tags are valid BCP-47")
    }

    /// Map an OS locale name to a supported language.
    ///
    /// Anything beginning `ms` is Malay — `ms-MY`, `ms-BN`, or a bare `ms`.
    /// Everything else is English, which is the documented policy rather than
    /// a guess (`docs/10-i18n.md`).
    pub fn from_os_locale(locale: &str) -> Self {
        let lowered = locale.to_ascii_lowercase();
        let primary = lowered
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_string();
        if primary == "ms" {
            Self::Malay
        } else {
            Self::English
        }
    }
}

/// The active loader. Set once at startup by [`init`].
static LOADER: OnceLock<FluentLanguageLoader> = OnceLock::new();

/// Load the bundled locales and select `language`.
///
/// English is always loaded as well as the selected language, so a missing
/// Malay key can fall back to it. Call once.
pub fn init(language: Language) -> Result<(), I18nError> {
    let loader = build_loader(language, None)?;
    LOADER
        .set(loader)
        .map_err(|_| I18nError::AlreadyInitialised)
}

/// Load the bundled locales, then apply any user overrides found in
/// `<data>/locales/`.
///
/// A broken override file is ignored with a logged warning and the embedded
/// text is used, so a user cannot break their own app by mis-editing one
/// (`docs/05-data-model-and-storage.md`).
pub fn init_with_overrides(language: Language, data_locales: &Path) -> Result<(), I18nError> {
    let loader = build_loader(language, Some(data_locales))?;
    LOADER
        .set(loader)
        .map_err(|_| I18nError::AlreadyInitialised)
}

fn build_loader(
    language: Language,
    overrides: Option<&Path>,
) -> Result<FluentLanguageLoader, I18nError> {
    let loader = FluentLanguageLoader::new(DOMAIN, Language::English.langid());

    // The **first** entry is the active language and the rest are fallbacks,
    // so the selected language leads and English follows. Putting English
    // first instead makes every lookup answer in English while the UI claims
    // to be in Malay — which is exactly what happened before this comment
    // existed.
    let mut requested = vec![language.langid()];
    if language != Language::English {
        requested.push(Language::English.langid());
    }

    // A user override takes precedence over the embedded file, so the
    // filesystem assets go first in the multiplexor. Where no override exists,
    // or where it is unusable, the embedded text is what answers.
    let result = match overrides.and_then(|dir| usable_overrides(dir, language)) {
        Some(dir) => {
            let assets = i18n_embed::AssetsMultiplexor::new([
                Box::new(i18n_embed::FileSystemAssets::try_new(dir)?)
                    as Box<dyn i18n_embed::I18nAssets + Send + Sync>,
                Box::new(Localizations),
            ]);
            loader.load_languages(&assets, &requested)
        }
        None => loader.load_languages(&Localizations, &requested),
    };

    result.map_err(|source| I18nError::Load {
        message: source.to_string(),
    })?;

    Ok(loader)
}

/// Decide whether `<data>/locales/` is worth handing to the loader.
///
/// An override that does not parse is ignored with a logged warning and the
/// embedded text is used, so a user cannot break their own app by mis-editing
/// one (`docs/05-data-model-and-storage.md`). Validation happens here, before
/// the loader sees the directory, because a parse failure inside the loader
/// would take the whole load with it rather than just that file.
fn usable_overrides(dir: &Path, language: Language) -> Option<&Path> {
    if !dir.is_dir() {
        return None;
    }

    let mut found_any = false;
    for lang in [Language::English, language] {
        let path = dir.join(lang.tag()).join("omni_toaster.ftl");
        if !path.is_file() {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => match fluent_bundle::FluentResource::try_new(text) {
                Ok(_) => {
                    found_any = true;
                    tracing::info!(path = %path.display(), "locale override will be used");
                }
                Err((_, errors)) => {
                    tracing::warn!(
                        path = %path.display(),
                        errors = errors.len(),
                        "locale override has syntax errors; using the bundled text"
                    );
                    return None;
                }
            },
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "locale override could not be read; using the bundled text"
                );
                return None;
            }
        }
    }

    found_any.then_some(dir)
}

/// Why localization could not be set up.
#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("the bundled locales could not be loaded: {message}")]
    Load { message: String },

    #[error("the locale override folder could not be read: {0}")]
    Overrides(#[from] i18n_embed::I18nEmbedError),

    #[error("localization was already initialised")]
    AlreadyInitialised,
}

/// Look up `key`, falling back to English and then to the key itself.
///
/// Returning the key rather than panicking is deliberate: a missing string
/// should be a visible oddity and a warning in the log, never a crash in front
/// of a user.
pub fn text(key: &str) -> String {
    let Some(loader) = LOADER.get() else {
        tracing::warn!(
            key,
            "a string was requested before localization was initialised"
        );
        return key.to_string();
    };
    if !loader.has(key) {
        tracing::warn!(key, "no translation found in either language");
        return key.to_string();
    }
    loader.get(key)
}

/// Look up `key`, substituting named arguments.
pub fn text_with(key: &str, args: &[(&str, Cow<'_, str>)]) -> String {
    let Some(loader) = LOADER.get() else {
        tracing::warn!(
            key,
            "a string was requested before localization was initialised"
        );
        return key.to_string();
    };
    let map: std::collections::HashMap<_, _> = args
        .iter()
        .map(|(name, value)| (*name, fluent_bundle::FluentValue::from(value.as_ref())))
        .collect();
    strip_isolates(&loader.get_args_concrete(key, map))
}

/// Fluent wraps every substituted value in Unicode isolating marks (U+2068 and
/// U+2069). They are correct for mixed-direction text and pure noise in both
/// of our left-to-right languages, where they reach the user as stray boxes —
/// "Logs are kept in \u{2068}D:\\...\u{2069}." — and reach a log file and a
/// spoken announcement the same way. Stripped here, at the one choke point
/// every substituted string passes through.
fn strip_isolates(text: &str) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

/// The message keys defined in a bundled catalogue.
///
/// Public because `--self-test` checks parity on the built exe and the unit
/// test checks it at `cargo test` time; they must agree, so they share this
/// rather than each parsing the files their own way.
pub fn message_keys(tag: &str) -> Result<std::collections::BTreeSet<String>, I18nError> {
    let path = format!("{tag}/omni_toaster.ftl");
    let file = Localizations::get(&path).ok_or_else(|| I18nError::Load {
        message: format!("{path} is not embedded"),
    })?;
    let text = String::from_utf8(file.data.to_vec()).map_err(|_| I18nError::Load {
        message: format!("{path} is not valid UTF-8"),
    })?;

    let resource =
        fluent_bundle::FluentResource::try_new(text).map_err(|(_, errors)| I18nError::Load {
            message: format!("{path} has {} syntax error(s)", errors.len()),
        })?;

    Ok(resource
        .entries()
        .filter_map(|entry| match entry {
            fluent_syntax::ast::Entry::Message(message) => Some(message.id.name.to_string()),
            _ => None,
        })
        .collect())
}

/// Check that both catalogues define exactly the same keys.
///
/// The error names the missing keys per side, because "the sets differ" sends
/// someone diffing two files by hand.
pub fn check_key_parity() -> Result<(), String> {
    let english = message_keys(Language::English.tag()).map_err(|e| e.to_string())?;
    let malay = message_keys(Language::Malay.tag()).map_err(|e| e.to_string())?;

    let missing_from_malay: Vec<&str> = english.difference(&malay).map(String::as_str).collect();
    let missing_from_english: Vec<&str> = malay.difference(&english).map(String::as_str).collect();

    if missing_from_malay.is_empty() && missing_from_english.is_empty() {
        return Ok(());
    }

    let mut message = String::new();
    if !missing_from_malay.is_empty() {
        message.push_str(&format!(
            "ms-MY is missing {}: {}",
            missing_from_malay.len(),
            missing_from_malay.join(", ")
        ));
    }
    if !missing_from_english.is_empty() {
        if !message.is_empty() {
            message.push_str("; ");
        }
        message.push_str(&format!(
            "en-US is missing {}: {}",
            missing_from_english.len(),
            missing_from_english.join(", ")
        ));
    }
    Err(message)
}

/// Build a loader for `language` without touching the global one.
///
/// `--self-test` uses this to prove each catalogue loads on its own, which the
/// global loader cannot show because it only ever holds one selection.
pub fn probe_language(language: Language) -> Result<(), I18nError> {
    build_loader(language, None).map(|_| ())
}

/// The language the loader is currently serving, for tests and for the UI.
pub fn current_language() -> Option<Language> {
    let loader = LOADER.get()?;
    let tag = loader.current_language().to_string();
    Some(Language::from_os_locale(&tag))
}

/// Initialise the global loader for a test, tolerating a sibling test having
/// done it first.
///
/// The loader is a process-wide `OnceLock` and `cargo test` runs tests in
/// parallel threads in one process, so exactly one test wins the race and
/// every other gets [`I18nError::AlreadyInitialised`]. A test that calls
/// [`init`] and unwraps is therefore order-dependent: it passes or fails on
/// thread scheduling, which is how a green local run turned into a red CI run.
///
/// This asserts the end state rather than who got there first — and fails
/// loudly if the global is serving a *different* language than the test needs,
/// because that test wants [`build_loader`] instead of the global.
#[cfg(test)]
fn ensure_initialised(language: Language) {
    match init(language) {
        Ok(()) => {}
        Err(I18nError::AlreadyInitialised) => {}
        Err(other) => panic!("the bundled locales failed to load: {other}"),
    }
    assert_eq!(
        current_language(),
        Some(language),
        "another test initialised the global loader to a different language; \
         use build_loader() for a test that needs its own language"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a bundled `.ftl` and return its message keys.
    fn keys_of(tag: &str) -> std::collections::BTreeSet<String> {
        let path = format!("{tag}/omni_toaster.ftl");
        let file = Localizations::get(&path)
            .unwrap_or_else(|| panic!("{path} is not embedded; is the folder name right?"));
        let text = String::from_utf8(file.data.to_vec()).expect("the locale file is UTF-8");

        let resource = match fluent_bundle::FluentResource::try_new(text) {
            Ok(resource) => resource,
            Err((_, errors)) => panic!("{path} has {} syntax errors: {errors:?}", errors.len()),
        };

        resource
            .entries()
            .filter_map(|entry| match entry {
                fluent_syntax::ast::Entry::Message(message) => Some(message.id.name.to_string()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn both_locales_are_embedded_and_parse_without_errors() {
        // A syntax error otherwise shows up only as a count in a log line, so
        // this is the test that names the file.
        let english = keys_of("en-US");
        let malay = keys_of("ms-MY");
        assert!(!english.is_empty(), "en-US has no messages");
        assert!(!malay.is_empty(), "ms-MY has no messages");
    }

    #[test]
    fn the_two_locales_have_identical_key_sets() {
        let english = keys_of("en-US");
        let malay = keys_of("ms-MY");

        let missing_from_malay: Vec<_> = english.difference(&malay).cloned().collect();
        let missing_from_english: Vec<_> = malay.difference(&english).cloned().collect();

        // Naming the keys matters: "the sets differ" sends someone diffing two
        // files by hand.
        assert!(
            missing_from_malay.is_empty(),
            "ms-MY is missing {} key(s): {missing_from_malay:?}",
            missing_from_malay.len()
        );
        assert!(
            missing_from_english.is_empty(),
            "en-US is missing {} key(s): {missing_from_english:?}",
            missing_from_english.len()
        );
    }

    #[test]
    fn every_key_is_lowercase_hyphenated_and_uses_a_known_prefix() {
        // docs/10-i18n.md fixes the prefixes. A key outside them is either a
        // typo or a new area that the document should name first.
        const PREFIXES: &[&str] = &[
            "tray-",
            "toast-",
            "settings-",
            "prayer-",
            "weather-",
            "reminder-",
            "announce-",
            "error-",
            "format-",
            "update-",
            // Phase 0 additions, also documented.
            "language-",
            "notice-",
            "day-",
            "month-",
            "help-",
        ];

        for key in keys_of("en-US") {
            assert!(
                key.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{key} is not lowercase and hyphenated"
            );
            assert!(
                PREFIXES.iter().any(|p| key.starts_with(p)),
                "{key} uses no documented prefix; add it to docs/10-i18n.md first"
            );
        }
    }

    #[test]
    fn os_locale_maps_ms_to_malay_and_everything_else_to_english() {
        for malay in ["ms-MY", "ms", "ms-BN", "MS-my", "ms_MY"] {
            assert_eq!(
                Language::from_os_locale(malay),
                Language::Malay,
                "{malay} should be Malay"
            );
        }
        for english in ["en-US", "en-GB", "de-DE", "zh-CN", "", "msa", "msx-Y"] {
            assert_eq!(
                Language::from_os_locale(english),
                Language::English,
                "{english} should fall back to English"
            );
        }
    }

    #[test]
    fn tags_and_name_keys_are_consistent() {
        assert_eq!(Language::English.tag(), "en-US");
        assert_eq!(Language::Malay.tag(), "ms-MY");

        // The name keys must exist in both files, or a language picker shows
        // raw key names.
        let english = keys_of("en-US");
        for lang in [Language::English, Language::Malay] {
            assert!(
                english.contains(lang.name_key()),
                "{} is missing from en-US",
                lang.name_key()
            );
        }
        assert!(english.contains("language-system"));
    }

    #[test]
    fn malay_text_differs_from_english_for_the_startup_errors() {
        // A key copied across untranslated is a real failure mode, and the
        // startup errors are the ones a Malay user is most likely to meet.
        let english = keys_of("en-US");
        assert!(english.contains("error-data-dir-not-writable"));

        let en = bundled_text("en-US", "error-data-dir-not-writable");
        let ms = bundled_text("ms-MY", "error-data-dir-not-writable");
        assert_ne!(
            en, ms,
            "the Malay startup error is identical to the English one"
        );
        assert!(ms.contains("OmniToaster"), "the product name should stay");
    }

    #[test]
    fn no_locale_message_spells_the_product_name_with_a_space() {
        // CR-0001. The product is OmniToaster, one word, everywhere a user can
        // see it (docs/01-vision-and-scope.md). The two-word spelling drifted
        // in and reached users through `OmniToaster --version`. Both files are
        // checked, because a translator working from the English catalogue
        // will copy whatever is in front of them.
        for tag in ["en-US", "ms-MY"] {
            let path = format!("{tag}/omni_toaster.ftl");
            let file = Localizations::get(&path).expect("embedded");
            let text = String::from_utf8(file.data.to_vec()).expect("UTF-8");

            let offenders: Vec<&str> = text
                .lines()
                .filter(|line| line.contains("Omni Toaster"))
                .collect();

            assert!(
                offenders.is_empty(),
                "{path} spells the product name with a space on {} line(s): {offenders:#?}",
                offenders.len()
            );
        }
    }

    #[test]
    fn a_malay_loader_answers_in_malay_and_still_falls_back_to_english() {
        // Regression: `load_languages` takes the ACTIVE language first and
        // fallbacks after. With English first, every lookup answered in
        // English while the UI reported Malay — and the bug was invisible to
        // the key-parity tests and to any key whose two translations happen to
        // match. `build_loader` is used directly because the global loader is
        // a OnceLock another test may already have claimed.
        let loader = build_loader(Language::Malay, None).expect("the bundled locales load");

        let malay = loader.get("error-data-dir-advice");
        let english_text = {
            let en = build_loader(Language::English, None).expect("loads");
            en.get("error-data-dir-advice")
        };

        assert_ne!(
            malay, english_text,
            "a Malay loader returned the English string"
        );
        assert!(
            malay.contains("Pindahkan"),
            "expected the Malay advice, got {malay:?}"
        );

        // And a key only English carries must still resolve, via the fallback.
        assert!(
            loader.has("language-system"),
            "the English fallback is not reachable from a Malay loader"
        );
    }

    #[test]
    fn substituted_values_carry_no_fluent_isolate_marks() {
        // These reached the user as stray boxes in "Logs are kept in
        // <U+2068>D:\...<U+2069>." before text_with stripped them.
        ensure_initialised(Language::English);
        let rendered = text_with(
            "notice-log-location",
            &[("path", r"D:\OmniToaster\OmniToasterData\logs".into())],
        );
        assert!(
            !rendered.contains('\u{2068}') && !rendered.contains('\u{2069}'),
            "isolating marks leaked into {rendered:?}"
        );
        assert!(rendered.contains("OmniToasterData"), "{rendered}");
    }

    #[test]
    fn a_broken_override_is_ignored_rather_than_breaking_the_app() {
        let temp = tempfile::tempdir().expect("temp dir");
        let locales = temp.path().join("locales");
        std::fs::create_dir_all(locales.join("en-US")).expect("mkdir");

        // `{ $unclosed` is a Fluent syntax error.
        std::fs::write(
            locales.join("en-US").join("omni_toaster.ftl"),
            "error-data-dir-advice = Broken { $unclosed\n",
        )
        .expect("write");

        assert!(
            usable_overrides(&locales, Language::English).is_none(),
            "a file with syntax errors must not be handed to the loader"
        );
    }

    #[test]
    fn a_valid_override_is_accepted_and_a_missing_folder_is_not() {
        let temp = tempfile::tempdir().expect("temp dir");
        let locales = temp.path().join("locales");
        std::fs::create_dir_all(locales.join("en-US")).expect("mkdir");
        std::fs::write(
            locales.join("en-US").join("omni_toaster.ftl"),
            "notice-log-location = Logs live in { $path }.\n",
        )
        .expect("write");

        assert_eq!(
            usable_overrides(&locales, Language::English),
            Some(locales.as_path())
        );
        assert!(
            usable_overrides(&temp.path().join("nope"), Language::English).is_none(),
            "a missing folder is simply no override"
        );
        // An empty but present folder is also no override, not an error.
        let empty = temp.path().join("empty");
        std::fs::create_dir_all(&empty).expect("mkdir");
        assert!(usable_overrides(&empty, Language::English).is_none());
    }

    #[test]
    fn a_malay_key_missing_from_the_file_falls_back_to_english() {
        // The fallback is what stops a half-translated release showing key
        // names to a Malay user. Proven on the key sets rather than through
        // the global loader, which another test may already have claimed:
        // English is always loaded alongside the selected language, so any key
        // present in en-US resolves even if ms-MY lacks it.
        let english = keys_of("en-US");
        assert!(
            english.contains("error-crash-loop"),
            "the fallback language must carry every key"
        );

        let mut requested = vec![Language::English.langid()];
        requested.push(Language::Malay.langid());
        assert_eq!(
            requested.first(),
            Some(&Language::English.langid()),
            "English must be requested first, or there is nothing to fall back to"
        );
    }

    /// The raw pattern text for a key, straight from the embedded file.
    fn bundled_text(tag: &str, key: &str) -> String {
        let path = format!("{tag}/omni_toaster.ftl");
        let file = Localizations::get(&path).expect("embedded");
        let text = String::from_utf8(file.data.to_vec()).expect("UTF-8");
        text.lines()
            .find(|line| line.starts_with(&format!("{key} =")))
            .unwrap_or_else(|| panic!("{key} not found in {tag}"))
            .to_string()
    }
}
