//! Every sentence the Updates pane can say, in both languages (FEAT-0080).
//!
//! One test in its own process, because it switches the process-wide language.

use ot_i18n::Language;
use ot_ui::update_pane::problem_text;
use ot_ui::update_service::Problem;
use semver::Version;

fn every_problem() -> Vec<Problem> {
    vec![
        Problem::Network,
        Problem::NoRelease,
        Problem::Rejected,
        Problem::BadDownload,
        Problem::FolderNotWritable,
        Problem::TooOld(Version::new(1, 2, 3)),
        Problem::RolledBack,
        Problem::Other,
    ]
}

#[test]
fn every_failure_reads_as_a_sentence_in_both_languages() {
    ot_i18n::init(Language::English).expect("installs");
    let english: Vec<String> = every_problem()
        .iter()
        .map(|problem| problem_text(problem, "git.zacx.net"))
        .collect();
    ot_i18n::set_language(Language::Malay).expect("switches");
    let malay: Vec<String> = every_problem()
        .iter()
        .map(|problem| problem_text(problem, "git.zacx.net"))
        .collect();

    for (en, ms) in english.iter().zip(&malay) {
        for text in [en, ms] {
            assert!(!text.is_empty() && !text.starts_with("update-"), "{text}");
            assert!(!text.contains('{'), "an unfilled variable: {text}");
        }
        assert_ne!(en, ms, "the Malay text is not the English one: {en}");
    }
    assert!(
        english[0].contains("git.zacx.net"),
        "the source is named: {}",
        english[0]
    );
    assert!(malay[0].contains("git.zacx.net"));
    assert!(english[5].contains("1.2.3") && malay[5].contains("1.2.3"));
}
