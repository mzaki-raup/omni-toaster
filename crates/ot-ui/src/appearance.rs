//! Settings ▸ Appearance: the toast's look and where it appears.
//!
//! Every control edits one [`Prefs`], and the preview is drawn from
//! `Prefs::spec` by `ot_toast::render::frame` and `ot_toast::content::draw` —
//! the same function and the same two painters a real toast goes through.
//! That is what "the preview matches the real toast" rests on: there is no
//! second drawing of a toast anywhere in this file.

use ot_toast::{Position, Prefs, Rgba, Shadow, Shape};

use crate::layout::{self, Columns};
use crate::theme::Tokens;

/// The style controls. Returns whether anything changed.
pub fn style_controls(ui: &mut egui::Ui, columns: Columns, prefs: &mut Prefs) -> bool {
    let mut changed = false;
    let width = columns.field;
    let style = &mut prefs.style;

    layout::labelled_row(ui, columns, &ot_i18n::text("settings-toast-shape"), |ui| {
        egui::ComboBox::from_id_salt("toast-shape")
            .width(width)
            .height(layout::field_height(ui))
            .selected_text(ot_i18n::text(style.shape.name_key()))
            .show_ui(ui, |ui| {
                for shape in Shape::all() {
                    changed |= ui
                        .selectable_value(&mut style.shape, shape, ot_i18n::text(shape.name_key()))
                        .changed();
                }
            });
    });
    ui.add_space(layout::ROW_GAP);

    // The radius only means something on a rounded rectangle: a rectangle is
    // always square and a pill is always round. Disabled rather than hidden,
    // so the form does not jump as the shape changes.
    let rounded = style.shape == Shape::RoundedRectangle;
    changed |= slider_row(
        ui,
        columns,
        "settings-toast-radius",
        &mut style.radius,
        ot_toast::spec::RADIUS,
        " px",
        rounded,
    );
    changed |= slider_row(
        ui,
        columns,
        "settings-toast-width",
        &mut style.width,
        ot_toast::spec::WIDTH,
        " px",
        true,
    );
    changed |= slider_row(
        ui,
        columns,
        "settings-toast-opacity",
        &mut style.opacity,
        ot_toast::spec::OPACITY,
        " %",
        true,
    );
    changed |= slider_row(
        ui,
        columns,
        "settings-toast-border",
        &mut style.border,
        ot_toast::spec::BORDER,
        " px",
        true,
    );

    layout::labelled_row(ui, columns, &ot_i18n::text("settings-toast-shadow"), |ui| {
        egui::ComboBox::from_id_salt("toast-shadow")
            .width(width)
            .height(layout::field_height(ui))
            .selected_text(ot_i18n::text(style.shadow.name_key()))
            .show_ui(ui, |ui| {
                for shadow in Shadow::all() {
                    changed |= ui
                        .selectable_value(
                            &mut style.shadow,
                            shadow,
                            ot_i18n::text(shadow.name_key()),
                        )
                        .changed();
                }
            });
    });
    ui.add_space(layout::ROW_GAP);

    changed |= colour_row(
        ui,
        columns,
        "settings-toast-background",
        &mut style.background,
    );
    changed |= colour_row(ui, columns, "settings-toast-accent", &mut style.accent);

    // Three options on one row, spaced so each reads as its own choice.
    let header = &mut prefs.header;
    layout::labelled_row(ui, columns, &ot_i18n::text("settings-toast-header"), |ui| {
        changed |= ui
            .checkbox(
                &mut header.icon,
                ot_i18n::text("settings-toast-header-icon"),
            )
            .changed();
        ui.add_space(layout::OPTION_GAP);
        changed |= ui
            .checkbox(
                &mut header.app_name,
                ot_i18n::text("settings-toast-header-app-name"),
            )
            .changed();
        ui.add_space(layout::OPTION_GAP);
        changed |= ui
            .checkbox(
                &mut header.timestamp,
                ot_i18n::text("settings-toast-header-time"),
            )
            .changed();
    });
    ui.add_space(layout::ROW_GAP);

    changed
}

/// The position controls: the picker, and for a custom point its two
/// percentages and the map it can be dragged on.
///
/// The map and the percentages edit **one** value. Neither can disagree with
/// the other because there is nothing for them to disagree about.
pub fn position_controls(
    ui: &mut egui::Ui,
    columns: Columns,
    prefs: &mut Prefs,
    work_area: egui::Rect,
    tokens: Tokens,
) -> bool {
    let mut changed = false;
    let position = &mut prefs.behaviour.position;

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-toast-position"),
        |ui| {
            egui::ComboBox::from_id_salt("toast-position")
                .width(columns.field)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(position.name_key()))
                .show_ui(ui, |ui| {
                    for preset in Position::presets() {
                        changed |= ui
                            .selectable_value(position, preset, ot_i18n::text(preset.name_key()))
                            .changed();
                    }
                    // Custom starts where the current preset puts the toast, so
                    // choosing it moves nothing until the user drags.
                    let custom_label =
                        ot_i18n::text(Position::Custom { x: 0.0, y: 0.0 }.name_key());
                    if ui
                        .selectable_label(position.is_custom(), custom_label)
                        .clicked()
                        && !position.is_custom()
                    {
                        *position = Position::Custom { x: 0.9, y: 0.9 };
                        changed = true;
                    }
                });
        },
    );
    ui.add_space(layout::ROW_GAP);

    if let Position::Custom { x, y } = position {
        let mut x_percent = *x * 100.0;
        let mut y_percent = *y * 100.0;
        layout::labelled_row(
            ui,
            columns,
            &ot_i18n::text("settings-toast-position-custom"),
            |ui| {
                ui.label(ot_i18n::text("settings-toast-position-x"));
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut x_percent)
                            .range(0.0..=100.0)
                            .speed(0.5)
                            .suffix(" %"),
                    )
                    .changed();
                ui.add_space(layout::OPTION_GAP);
                ui.label(ot_i18n::text("settings-toast-position-y"));
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut y_percent)
                            .range(0.0..=100.0)
                            .speed(0.5)
                            .suffix(" %"),
                    )
                    .changed();
            },
        );
        *x = x_percent / 100.0;
        *y = y_percent / 100.0;
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("toast-position-custom-hint"),
            tokens.text_dim,
        );
        ui.add_space(layout::ROW_GAP);
    }

    changed |= position_map(ui, columns, prefs, work_area, tokens);
    changed
}

/// A small map of the monitor: nine dots for the presets, and the toast
/// drawn where it will appear. Click a dot to choose a preset; drag anywhere
/// to place the toast at a custom point.
fn position_map(
    ui: &mut egui::Ui,
    columns: Columns,
    prefs: &mut Prefs,
    work_area: egui::Rect,
    tokens: Tokens,
) -> bool {
    let mut changed = false;
    let map_width = columns.field.min(320.0);
    let aspect = (work_area.height() / work_area.width().max(1.0)).clamp(0.3, 1.5);
    let size = egui::vec2(map_width, map_width * aspect);

    ui.horizontal(|ui| {
        ui.add_space(columns.label);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, egui::CornerRadius::same(4), tokens.surface_raised);
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(4),
            egui::Stroke::new(1.0_f32, tokens.border),
            egui::StrokeKind::Inside,
        );

        // Scale the toast into the map, and place it with the same function
        // a real toast uses, so the map cannot lie about where it will go.
        let scale = rect.width() / work_area.width().max(1.0);
        let toast_size = egui::vec2(
            prefs.style.width,
            prefs
                .style
                .height
                .unwrap_or(ot_toast::viewport::AUTO_HEIGHT),
        );
        let at = ot_toast::placement::place(
            work_area,
            toast_size,
            prefs.behaviour.position,
            0,
            prefs.behaviour.stack_gap,
        );
        let mini =
            egui::Rect::from_min_size(rect.min + (at - work_area.min) * scale, toast_size * scale);
        painter.rect_filled(
            mini,
            egui::CornerRadius::same(2),
            ot_toast::render::colour(prefs.style.accent, 0.85),
        );

        // The nine preset dots. A click near one chooses it.
        let mut nearest: Option<(f32, Position)> = None;
        let pointer = response.interact_pointer_pos();
        for preset in Position::presets() {
            let dot_at = ot_toast::placement::place(work_area, toast_size, preset, 0, 0.0);
            let centre = rect.min + (dot_at - work_area.min) * scale + toast_size * scale / 2.0;
            let selected = prefs.behaviour.position == preset;
            painter.circle_filled(
                centre,
                if selected { 5.0 } else { 3.5 },
                if selected {
                    tokens.accent
                } else {
                    tokens.text_dim
                },
            );
            if let Some(pointer) = pointer {
                let distance = pointer.distance(centre);
                if nearest.is_none_or(|(best, _)| distance < best) {
                    nearest = Some((distance, preset));
                }
            }
        }

        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            // Dragging places a custom point, as a fraction of the work area.
            let x = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let y = ((pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
            prefs.behaviour.position = Position::Custom { x, y };
            changed = true;
        } else if response.clicked()
            && let Some((distance, preset)) = nearest
            && distance < 14.0
        {
            prefs.behaviour.position = preset;
            changed = true;
        }
    });
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("settings-toast-map-hint"),
        tokens.text_dim,
    );
    changed
}

/// Draw the preview: the same spec, the same two painters, as a real toast.
pub fn preview(
    ui: &mut egui::Ui,
    columns: Columns,
    spec: &ot_toast::ToastSpec,
    tokens: Tokens,
    icon: Option<ot_toast::content::Picture>,
    image: Option<ot_toast::content::Picture>,
) {
    let height = spec.style.height.unwrap_or(ot_toast::viewport::AUTO_HEIGHT);
    // The preview is drawn at the toast's real size, so what fits here fits
    // there. A narrow window gets a scrollbar rather than a shrunken lie.
    let size = egui::vec2(spec.style.width, height);
    ui.horizontal(|ui| {
        ui.add_space(columns.label);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let content = ot_toast::render::frame(ui.painter(), rect, spec);
        ot_toast::content::draw(
            ui.painter(),
            content,
            spec,
            tokens.text,
            tokens.text_dim,
            icon,
            image,
        );
    });
}

fn slider_row(
    ui: &mut egui::Ui,
    columns: Columns,
    key: &str,
    value: &mut f32,
    range: ot_toast::spec::Range,
    unit: &str,
    enabled: bool,
) -> bool {
    let mut changed = false;
    let width = columns.field;
    layout::labelled_row(ui, columns, &ot_i18n::text(key), |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            ui.spacing_mut().slider_width = (width - 80.0).max(80.0);
            changed = ui
                .add(
                    egui::Slider::new(value, range.min..=range.max)
                        .suffix(unit)
                        // Whole pixels and whole percent: egui picks its
                        // decimals from the range, which gave "0.000 px"
                        // beside "12.0 px" on the same form.
                        .fixed_decimals(0),
                )
                .changed();
        });
    });
    ui.add_space(layout::ROW_GAP);
    changed
}

fn colour_row(ui: &mut egui::Ui, columns: Columns, key: &str, rgba: &mut Rgba) -> bool {
    let (r, g, b, a) = rgba.parts();
    let mut colour = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
    let mut changed = false;
    layout::labelled_row(ui, columns, &ot_i18n::text(key), |ui| {
        changed = egui::color_picker::color_edit_button_srgba(
            ui,
            &mut colour,
            egui::color_picker::Alpha::Opaque,
        )
        .changed();
        // The hex value is readable beside the swatch (docs/04-ui-design.md).
        ui.label(format!("#{r:02x}{g:02x}{b:02x}"));
    });
    if changed {
        let [r, g, b, a] = colour.to_srgba_unmultiplied();
        *rgba = Rgba::new(r, g, b, a);
    }
    ui.add_space(layout::ROW_GAP);
    changed
}
