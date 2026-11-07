# UI and UX rules

One UI language across every tool. 

## Shared components (use these, don't re-roll them)

- Never a bare `TableBuilder` for a wide table — use `DataTable`.
- **An empty table says so:** an editable or read-only table with no rows
  shows its header and a one-line sentence-case hint under it ("This
  table has no indexes yet."), never a bare header.
- **A label is shown whole:** a button or card that shrinks to fit a row
  never goes narrower than its longest label; past that the row wraps.
  An ellipsis is only for user data that can be any length, never for
  the app's own names while there is room.
- **Menu items carry a glyph:** every item of a right-click menu we build
  has a glyph before its label (Remove and Delete a trash can, Clear a
  broom, Open an arrow), the same glyph for the same verb in every
  activity; an item that cannot act.
- **A label beside a button sits on its middle line too:** The row is a `centred_row`, label and button on one middle
  line.
- **Options on one row are spaced apart:** checkboxes or radio buttons
  sharing a row have `form::OPTION_GAP` (18 px) between them, so each reads
  as its own choice; options in a form grid stay one per row.

## Text

- Every visible string goes through Fluent, `locales/en.ftl` and `locales/ms-MY`. **Edit
  locale files with the Edit tool, never a heredoc** — an unquoted heredoc eats
  variable placeholders and silently drops the message, and
  `the_bundled_locale_parses_without_junk` then fails the gate.
- Casing:
  - **Title Case** for button labels, menu and list items, setting names, tab
    and category labels. Minor words (a, the, of, to, from, at, in, on, and)
    stay lowercase.
  - **Sentence case** for tooltips, hints, placeholders, dialog bodies, toasts
    and empty states.
  - A parenthetical attached to a label starts with a capital — "None (Direct
    connection)". A parenthetical inside a sentence stays lowercase.

- **One height for every input in a row:** a text field, a dropdown, a
  pick field, a checkbox and a button beside them are all
  `form::field_height(ui)` tall (about 32 px at the default font) (CR-0026).
  - Text fields come from `form::text_field`, `form::text_field_width`,
    `form::pick_field` or, when the call site needs its own width, hint or
    font, `form::line_edit(&mut value)`. **Never** a bare
    `egui::TextEdit::singleline` - egui's default margin draws it about ten
    pixels shorter (CR-0061).
  - Dropdowns, checkboxes and buttons get the height from their container:
    `form::field_grid` (forms), `toolbar::strip_style` (toolbars),
    `table::edit_table` (editable tables). Anywhere else a row mixes a
    label, a field, a dropdown or buttons, open it with
    `form::centred_row(ui, |ui| …)`, or `form::centred_row_wrapped` when
    it may wrap (CR-0026, CR-0027, CR-0061).
  - **Labels sit on the field's middle line.** egui fixes a horizontal
    row's height when the row opens, so the field height must be set
    *before* `ui.horizontal` / `ui.horizontal_wrapped`, never inside the
    closure: `match_field_height` inside the closure makes the field tall
    but leaves the label centred on the old, shorter row - it sits high.
    `centred_row` does it in the right order. A bare
    `ui.horizontal` holding a label and a `line_edit` is the same bug.
- **Width shows the value:** a field is wide enough for the values it
  holds. In a form a field is `FIELD_MIN_WIDTH` (260) to `FIELD_MAX_WIDTH`
  (560) wide. In a table, a column's width is its floor and spare width is
  shared out (`table::edit_table` does this); floors are a name 200, a
  type 220, a default or an expression 180, a list of columns 200, a
  reference (`schema.table`) 220, a rule (`CASCADE`, `SET NULL`) 140, a
  checkbox 60.
- **Vertical middle:** every row that holds more than one input lines its
  inputs up on one middle line. `ui.horizontal` does this once the heights
  match; `ui.horizontal_top` is only for page layouts (a list beside a
  form), never for an input row. A label beside a field in a grid is
  `form::label`.
- **Description under a field** goes on its own row
  (`form::field_hint_row`), never in the field's cell, or the row grows and
  its label drops to the middle of the pair.

### Tables

- **Every table** has the header band (`table::header_band`, the surface
  fill and a rule under it), bold header text at `table::HEADER_TEXT`
  (14.5) on a `table::HEADER_HEIGHT` (34) row, data rows
  `table::ROW_HEIGHT` (26) tall (editable rows `table::edit_row_height`),
  striped rows, and every cell laid out by `table::cell_layout()` - from
  the left, **vertically centred** in its row.
- A table that scrolls on its own sits in `form::well`; a block of output
  in `form::scroll_well` with a fixed height.

### Buttons and toolbars

- **A row of buttons wraps** instead of running past the pane: build it
  with `ui.horizontal_wrapped`, never `ui.horizontal`, when it holds more
  than two buttons or sits beside a field.
- Hover highlights a control; it never grows or moves (`Theme::apply`
  zeroes widget expansion). This holds for every
  hand-painted row, card, tree node, link and button too: hover changes
  the fill or draws a border, it never paints a larger rect, a bigger font
  or a scaled icon, and no scope may reset the style to egui's default
  (which brings expansion back). 
  Selectable items go through `form::selectable_label` /
  `selectable_value` / `toggle_value` / `add_selectable`, never egui's own

### The right component for the value

| The value is | Use |
|---|---|
| on / off | a checkbox (`ui.checkbox`), or `form::toggle_button` in a toolbar |
| one of a fixed set | a dropdown (`egui::ComboBox`, raised to field height) |
| one the server knows, or typed | `form::pick_field` (typed or chosen) |
| a date | `hs_ui::datefield::date_field` |
| a secret (password, token, key) | `form::secret_field`, stored as `secret:<ulid>` |
| a number in a range | `form::slider`, or a `DragValue` beside its unit |
| a name, a path, a short text | `form::text_field` / `form::line_edit` |
| a list of name = value pairs | `hs_ui::kv_editor::KvEditor` |
| rows of several inputs | `table::edit_table` |
| facts to read (Type / Bug) | `form::detail_table` |
| many rows to read, sort, filter | `table::DataTable` |

A value is never squeezed into a weaker component (a yes/no as a text
field, a fixed set as free text) and never into a stronger one that the
user has to fight (a free name as a dropdown).

### Checking a screen

Run the app on a scratch data folder and look, at the default size and
at a narrow window. Every item below must hold before the screen is
reported; write what you checked in the item's App box.

1. **Fields fit their values:** every field and every column shows a
   typical value whole (a table name, a type such as
   `enum('low','normal','high')`, a URL, a path). Nothing is a sliver, and
   long values truncate with an ellipsis, not mid-glyph.
2. **One height, one middle line:** every input in a row is the field
   height, and the row's inputs, labels and buttons sit on one middle line.
   Table cells are vertically centred.
3. **Sections are separated:** each section of a work pane (a toolbar, a
   form, a table, a script or output area, a status line) is set apart by
   a separator or a card, with `PANE_GAP` above and below it; form rows
   are `ROW_GAP` apart.
4. **Margins:** nothing touches a pane edge; content is `PANE_MARGIN` in.
5. **Right components:** each value uses the component the table above
   names for it.
6. **Tables:** the header band, bold header text, centred cells, the
   standard row heights.
7. **Text:** Title Case labels and buttons, sentence-case hints and
   toasts; every string from Fluent.
8. **States:** the empty, loading and error states each say something.
9. **Buttons:** every button has an icon and a label (row actions: icon
   and tooltip), and every row of buttons wraps at a narrow window.

## Icons

Verify a Nerd Font / Phosphor glyph by looking at it on screen. A codepoint
being present does not mean it draws the right picture.

## Verifying UI work

Tests are not enough. Run the app against a scratch data folder and look:

```bash
cargo run -p omnitpaster -- --data-dir ./tmp-data
```

`egui_kittest` covers widget snapshots, but synthetic clicks never form
double-clicks — exercise double-click paths in the running app.
