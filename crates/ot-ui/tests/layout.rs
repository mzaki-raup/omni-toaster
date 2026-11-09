//! Layout tests of the real Appearance controls (FEAT-0024).
//!
//! `egui_kittest` drives the same functions the settings window calls, with
//! no window and no GPU, and every widget's rect is checked against the pane
//! it lives in: nothing may run past the right-hand edge, in either language,
//! or in the pseudo-locale, at 100%, 150% or 200%, at the default width or
//! the narrow minimum. That is the overflow half of Phase 2's exit criteria,
//! held by the gate instead of by someone remembering to look.
//!
//! In its own process, and as one test, because it changes the process-wide
//! language — the same reason as `ot-i18n`'s switching test.

use egui_kittest::Harness;
use egui_kittest::kittest::NodeT as _;
use ot_i18n::Language;
use ot_toast::Prefs;

/// The pane's width at the window's default size: 880 less the 180 px rail
/// and the margins either side.
const DEFAULT_PANE: f32 = 668.0;
/// And at the 560 px minimum the window allows.
const NARROW_PANE: f32 = 348.0;

/// The work area the position map draws: a 1080p screen with a taskbar.
fn work_area() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1032.0))
}

/// Lay the Appearance controls out and return every widget that ran past
/// the pane, with its label, so a failure says what overflowed.
fn overflowing(width: f32, scale: f32) -> Vec<String> {
    let mut prefs = Prefs::default();
    let mut harness = Harness::builder()
        .with_size([width, 4000.0])
        .with_pixels_per_point(scale)
        .build_ui(|ui| {
            let columns = ot_ui::layout::columns(ui);
            ot_ui::appearance::style_controls(ui, columns, &mut prefs);
            ot_ui::appearance::position_controls(
                ui,
                columns,
                &mut prefs,
                work_area(),
                ot_ui::Tokens::DARK,
            );
        });
    // The real font set: with `default_fonts` off, the harness's own
    // context would measure every glyph as zero wide and pass anything.
    harness
        .ctx
        .set_fonts(ot_ui::settings_window::font_definitions().expect("the Windows UI font"));
    harness.run();

    let pane = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 4000.0));
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        // Not `node.rect()`, which panics on a node with no bounds — a text
        // run or a container can have none. Such a node is not a widget that
        // could overflow, so it is skipped rather than failed.
        let Some(bounds) = node.accesskit_node().bounding_box() else {
            continue;
        };
        // Accesskit reports bounds in physical pixels and the pane is in
        // points, so they are brought to the same unit. The first version of
        // this test compared them as they came and reported 123 overflows at
        // 1.5x and 2x - every one of them the scale factor, none of them real.
        let rect = egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        ) / scale;
        if !rect.is_finite() || rect.width() <= 0.0 {
            continue;
        }
        // Half a point of slack for rounding at fractional scales.
        if rect.right() > pane.right() + 0.5 || rect.left() < pane.left() - 0.5 {
            let label = node.accesskit_node().label().unwrap_or_default();
            found.push(format!("{label:?} at {rect:?}"));
        }
    }
    found
}

/// The detection itself, on a widget that certainly overflows.
///
/// A layout test that cannot fail proves nothing, and this project has had
/// two that could not — one measuring text with no font installed, one
/// comparing pixels to points. This one must find a 500-point button in a
/// 300-point pane, at every scale, or the test above means nothing either.
#[test]
fn the_detection_catches_a_widget_that_really_overflows() {
    for scale in [1.0_f32, 1.5, 2.0] {
        let mut harness = Harness::builder()
            .with_size([300.0, 200.0])
            .with_pixels_per_point(scale)
            .build_ui(|ui| {
                ui.add_sized([500.0, 30.0], egui::Button::new("too wide"));
            });
        harness
            .ctx
            .set_fonts(ot_ui::settings_window::font_definitions().expect("the Windows UI font"));
        harness.run();

        let caught = harness.root().children_recursive().any(|node| {
            node.accesskit_node()
                .bounding_box()
                .map(|b| (b.x1 as f32) / scale > 300.5)
                .unwrap_or(false)
        });
        assert!(
            caught,
            "a 500-point button in a 300-point pane was not caught at {scale}x"
        );
    }
}

#[test]
fn the_appearance_controls_fit_their_pane_in_both_languages_at_every_scale() {
    ot_i18n::init(Language::English).expect("the first install");

    // The fonts measure something, or every assertion below is vacuous.
    let probe = {
        let ctx = egui::Context::default();
        ctx.set_fonts(ot_ui::settings_window::font_definitions().expect("the Windows UI font"));
        let mut width = 0.0;
        let _ = ctx.run(Default::default(), |ctx| {
            let galley = ctx.fonts_mut(|fonts| {
                fonts.layout_no_wrap(
                    "measure me".to_owned(),
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                )
            });
            width = galley.size().x;
        });
        width
    };
    assert!(
        probe > 10.0,
        "the font measured {probe}px, so nothing below would mean anything"
    );

    // English, Malay, then Malay as the pseudo-locale: about 40% longer
    // again, to find breakage Malay does not reach yet (`docs/10-i18n.md`).
    let mut failures = Vec::new();
    for (language, pseudo) in [
        (Language::English, false),
        (Language::Malay, false),
        (Language::Malay, true),
    ] {
        ot_i18n::set_language(language).expect("the switch");
        ot_i18n::set_pseudo(pseudo);
        // The switch reached the strings, or the pseudo pass proves nothing.
        assert_eq!(
            ot_i18n::text("settings-section-appearance").starts_with('['),
            pseudo
        );
        let name = if pseudo {
            "pseudo".to_owned()
        } else {
            format!("{language:?}")
        };
        for width in [DEFAULT_PANE, NARROW_PANE] {
            for scale in [1.0_f32, 1.5, 2.0] {
                for widget in overflowing(width, scale) {
                    failures.push(format!("{name}, {width}px pane, {scale}x: {widget}"));
                }
            }
        }
    }
    ot_i18n::set_pseudo(false);
    assert!(
        failures.is_empty(),
        "{} widgets ran past the pane:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
