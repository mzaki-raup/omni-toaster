//! Every prayer's label against the reference table in `docs/10-i18n.md`
//! (FEAT-0062, the Phase 5 exit criterion "prayer labels match the reference
//! table in both languages").
//!
//! One test in its own process, because it switches the process-wide language.

use ot_i18n::Language;

const DOC: &str = include_str!("../../../docs/10-i18n.md");

/// The rows of the table: key, English, Malay.
fn table() -> Vec<(String, String, String)> {
    DOC.lines()
        .filter(|line| line.starts_with("| `prayer-name-"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 3, "a malformed row: {line}");
            (
                cells[0].trim_matches('`').to_owned(),
                cells[1].to_owned(),
                cells[2].to_owned(),
            )
        })
        .collect()
}

#[test]
fn every_prayer_label_matches_the_reference_table_in_both_languages() {
    let rows = table();
    assert_eq!(rows.len(), 7, "the seven prayers with a name: {rows:?}");
    ot_i18n::init(Language::English).expect("installs");
    for (key, english, _) in &rows {
        assert_eq!(&ot_i18n::text(key), english, "{key} in English");
    }
    ot_i18n::set_language(Language::Malay).expect("switches");
    for (key, _, malay) in &rows {
        assert_eq!(&ot_i18n::text(key), malay, "{key} in Malay");
    }
}
