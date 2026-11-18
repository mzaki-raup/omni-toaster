//! Settings ▸ Animation: how a toast arrives and leaves (`docs/specs/toast.md`
//! §5, `docs/04-ui-design.md`).
//!
//! The controls edit one [`Prefs`]; Play in the pane fires a test toast with
//! them, built by the same function as every real toast. The pane has no
//! preview: an animation cannot be shown in a still one.

use ot_toast::{Easing, Effect, Prefs};

use crate::layout::{self, Columns};
use crate::theme::Tokens;

/// The animation controls. `windows_animations` is whether Windows' own
/// "Show animations" is on, stated under Reduce Motion. Returns whether
/// anything changed.
pub fn controls(
    ui: &mut egui::Ui,
    columns: Columns,
    prefs: &mut Prefs,
    windows_animations: bool,
    tokens: Tokens,
) -> bool {
    let mut changed = false;
    let animation = &mut prefs.animation;

    changed |= effect_row(
        ui,
        columns,
        "settings-animation-enter",
        "animation-enter",
        &mut animation.enter,
        &Effect::all_in(),
    );
    changed |= effect_row(
        ui,
        columns,
        "settings-animation-leave",
        "animation-leave",
        &mut animation.leave,
        &Effect::all_out(),
    );

    let range = ot_toast::spec::ANIMATION_MS;
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-animation-duration"),
        |ui| {
            ui.spacing_mut().slider_width = (columns.field - 90.0).max(80.0);
            changed |= ui
                .add(
                    egui::Slider::new(&mut animation.duration_ms, range.min..=range.max)
                        .suffix(" ms")
                        .step_by(10.0),
                )
                .changed();
        },
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-animation-easing"),
        |ui| {
            egui::ComboBox::from_id_salt("animation-easing")
                .width(columns.field)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(animation.easing.name_key()))
                .show_ui(ui, |ui| {
                    for easing in Easing::all() {
                        changed |= ui
                            .selectable_value(
                                &mut animation.easing,
                                easing,
                                ot_i18n::text(easing.name_key()),
                            )
                            .changed();
                    }
                });
        },
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-animation-reduce"),
        |ui| {
            changed |= ui.checkbox(&mut animation.reduce_motion, "").changed();
        },
    );
    // Windows' own setting, stated rather than hidden: either one reduces
    // motion, and someone wondering why a slide fades should see why.
    let reason = if animation.reduce_motion {
        "settings-animation-reduced"
    } else if windows_animations {
        "settings-animation-windows-on"
    } else {
        "settings-animation-windows-off"
    };
    layout::field_hint(ui, columns, &ot_i18n::text(reason), tokens.text_dim);
    ui.add_space(layout::ROW_GAP);

    changed
}

/// A dropdown of effects.
fn effect_row(
    ui: &mut egui::Ui,
    columns: Columns,
    label: &str,
    id: &str,
    value: &mut Effect,
    choices: &[Effect],
) -> bool {
    let mut changed = false;
    layout::labelled_row(ui, columns, &ot_i18n::text(label), |ui| {
        egui::ComboBox::from_id_salt(id)
            .width(columns.field)
            .height(layout::field_height(ui))
            .selected_text(ot_i18n::text(value.name_key()))
            .show_ui(ui, |ui| {
                for effect in choices {
                    changed |= ui
                        .selectable_value(value, *effect, ot_i18n::text(effect.name_key()))
                        .changed();
                }
            });
    });
    ui.add_space(layout::ROW_GAP);
    changed
}
