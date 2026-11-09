# OmniToaster — English (en-US)
#
# This file is the source of truth for every visible string. Keys are
# lowercase, hyphenated, and grouped by the prefixes in docs/10-i18n.md.
# Every key added here must be added to ms-MY in the same commit; the key
# parity test fails the build otherwise.
#
# Casing: Title Case for button labels, menu items, setting names and
# category labels. Sentence case for hints, placeholders, dialog bodies,
# toasts and empty states.
#
# Edit this file with an editor, never with a shell heredoc: an unquoted
# heredoc expands { $variables } and backticks and silently drops the
# message (docs/GOTCHAS.md).

## Startup and fatal errors. These are the only strings a user can see
## before the settings window exists, so they must be complete in both
## languages — a user whose app has given up should not be shown English
## they cannot read.

# $path is the folder the app tried to use.
error-data-dir-not-writable = OmniToaster cannot start because it cannot write to { $path }.
error-data-dir-advice = Move the app to a folder you can write to, or start it with --data-dir pointing at one. Nothing has been changed.
# $path is the folder that could not be created.
error-data-dir-not-creatable = OmniToaster could not create its data folder { $path }.
error-exe-location-unknown = OmniToaster could not work out where it is installed.
# $path is the folder the app writes its logs to.
notice-log-location = Logs are kept in { $path }.
# Shown when a second copy is started. Not an error: the app the user asked
# for is already running, so the second copy bows out quietly.
notice-already-running = OmniToaster is already running. Look for it in the notification area.
# $path is the crash report folder.
error-crash-loop = OmniToaster stopped restarting after repeated crashes. The crash reports are in { $path }.

## --help. Printed in the user's language, because someone running
## OmniToaster --help on a Malay machine is exactly the person who needs
## it in Malay.

help-usage = Usage: OmniToaster [options]
help-intro = Portable toast notifications for reminders, prayer times, weather and the time.
help-option-help = show this help and exit
help-option-version = show the version and exit
help-option-self-test = run the headless checks and exit, printing one line per check
help-option-worker = run as the worker process (not yet implemented; Phase 8)
help-option-data-dir = use this folder instead of OmniToasterData beside the executable
help-option-log-level = log at this level for this run: error, warn, info, debug or trace
help-option-lang = use this language for this run, overriding the Windows setting
help-footer = Everything OmniToaster writes stays in its data folder. Nothing is sent anywhere unless you turn it on.

## Language names, shown in Settings ▸ General ▸ Language. Each name is
## written in its own language, which is what users expect in a language
## picker.

language-system = Same as Windows
language-en-us = English
language-ms-my = Bahasa Melayu

## Toasts. Sentence case for the body, Title Case for the actions. Only
## built-in labels live here: a user's own reminder text is never a key.

toast-app-name = OmniToaster
toast-test-title = Test Toast
toast-test-body = This is what a toast looks like with your current settings.
toast-action-snooze = Snooze
toast-action-dismiss = Dismiss
toast-action-open = Open

## Where a toast appears: nine presets — four corners, four edges and the
## centre — plus the custom point. Shown in Settings ▸ Appearance. Title Case,
## because they are list items.

toast-position-top-left = Top Left
toast-position-top = Top
toast-position-top-right = Top Right
toast-position-left = Left
toast-position-centre = Centre
toast-position-right = Right
toast-position-bottom-left = Bottom Left
toast-position-bottom = Bottom
toast-position-bottom-right = Bottom Right
toast-position-custom = Custom Position
# Under the position picker, when Custom Position is chosen.
toast-position-custom-hint = Drag the preview to place it, or type the distance from the left and top edges as a percentage.

## Why an imported image was refused. Sentence case, and each one names the
## rule the file broke — "it didn't work" is not a message. $name is the
## file's own name.

error-import-too-large = { $name } is larger than 5 MB, so it was not imported.
error-import-not-an-image = { $name } is not a PNG, JPEG or WebP picture, whatever its name says.
error-import-too-many-pixels = { $name } is larger than 4096 by 4096 pixels, so it was not imported.
error-import-undecodable = { $name } could not be read as a picture. It may be damaged.
error-import-escapes = { $name } could not be copied into the OmniToaster folder.
error-import-unreadable = { $name } could not be read.

## Toast shapes and shadows, shown in Settings ▸ Appearance.

toast-shape-rectangle = Rectangle
toast-shape-rounded = Rounded Rectangle
toast-shape-pill = Pill
toast-shadow-off = Off
toast-shadow-soft = Soft
toast-shadow-strong = Strong

## The tray icon and its menu. Title Case for every item, because they are
## menu items. The glyph in front of each one is chosen in code, not here.

tray-open-settings = Open Settings
tray-pause = Pause All Notifications
tray-pause-30 = For 30 Minutes
tray-pause-60 = For 1 Hour
tray-pause-until-resumed = Until I Resume
tray-resume = Resume Notifications
tray-test-toast = Test Toast
tray-snooze-last = Snooze Last
tray-quit = Quit OmniToaster

## The tray tooltip. Sentence case: it is a hint, not a label. It carries the
## paused state, because an app someone has forgotten they muted is the worst
## outcome of a pause.

tray-tooltip-idle = OmniToaster is running.
# $time is the moment the pause lapses, formatted by ot_i18n.
tray-tooltip-paused-until = Notifications are paused until { $time }.
tray-tooltip-paused-until-resumed = Notifications are paused until you resume them.

## Quitting while a pause is running. Sentence case for the body; the buttons
## are Title Case, and each says what it does rather than Yes or No.

tray-quit-confirm-title = Quit OmniToaster?
tray-quit-confirm-body = Notifications are paused. Quitting now closes OmniToaster altogether, and nothing will be delivered until you start it again.
tray-quit-confirm-quit = Quit Anyway
tray-quit-confirm-stay = Keep Running

## The settings window: its title, the sections in the left rail, and the
## General pane. Title Case for the labels, sentence case for the hints.

settings-window-title = Settings
settings-section-general = General
settings-section-appearance = Appearance
# The section holding the image library: pictures uploaded once and used by any notification.
settings-section-pictures = Picture Library

## Settings ▸ Appearance. Title Case for the labels, sentence case for the
## hints.

settings-toast-preview = Preview
settings-toast-test = Show a Test Toast
settings-toast-shape = Shape
settings-toast-radius = Corner Radius
settings-toast-width = Width
settings-toast-opacity = Opacity
settings-toast-border = Border
settings-toast-shadow = Shadow
settings-toast-background = Background
settings-toast-accent = Accent Colour
settings-toast-header = Header
settings-toast-header-icon = Icon
settings-toast-header-app-name = App Name
settings-toast-header-time = Time
settings-toast-position = Position
settings-toast-position-custom = Custom Point
# How many toasts may be on screen at once (1 to 5); the rest wait their turn.
settings-toast-max-visible = Visible at Once
# The space between one toast and the next in a stack, in pixels.
settings-toast-stack-gap = Gap Between Toasts
settings-toast-position-x = From Left
settings-toast-position-y = From Top
settings-toast-map-hint = Click a dot to choose a position, or drag to place the toast anywhere on the screen.
settings-toast-test-content = Test Toast Content
settings-toast-test-content-hint = These only change the preview and the test toast. Every notification carries its own icon, image and accent colour.
settings-toast-icon = Icon
settings-toast-image = Image
settings-toast-image-border = Border Around the Image
settings-toast-own-accent = Own Accent Colour
# The titles of the Windows Open dialog.
settings-toast-icon-dialog-title = Choose an icon for the toast header
settings-toast-image-dialog-title = Choose an image for the toast
# The file-type filter in the Open dialog.
settings-toast-image-filter = Pictures (PNG, JPEG, WebP)
# Settings > Picture Library. The line under the section title.
settings-library-hint = Upload a picture once and use it in any notification, as its icon or its image.
# The button that opens the file dialog to add a picture to the library.
settings-library-upload = Upload a Picture
# The last item of every icon and image picker, which uploads a picture and chooses it.
settings-library-upload-new = Upload a New Picture
# The title of the Open dialog when uploading from the Picture Library section.
settings-library-dialog-title = Choose a picture for the library
# Shown while the library is being read.
settings-library-loading = Loading the pictures…
# Shown when the library has no pictures yet.
settings-library-empty = No pictures yet. Upload one to use it in a notification.
# Tooltip on the name field beside each picture.
settings-library-label-hint = The name this picture goes by in every picker
# The button that removes a picture from the library.
settings-library-remove = Remove
# Asked before removing a picture something still uses. $label is the picture's name.
settings-library-in-use = “{ $label }” is still in use. Remove it anyway? Whatever uses it falls back to the app icon or to no image.
# A line under that question, when the test toast under Appearance shows the picture.
settings-library-used-by-test-toast = The test toast uses it.
# A line under that question, when notification templates use the picture.
settings-library-used-by-templates = Notification templates that use it will lose it.
# Confirms that removal.
settings-library-remove-anyway = Remove Anyway
# Cancels that removal.
settings-library-keep = Keep It
# The first item of the icon picker: no picture of its own, so the app's icon.
settings-library-none-icon = App Icon
# The first item of the image picker: no image.
settings-library-none-image = No Image
settings-language = Language
# Under the language picker. The switch is immediate, so the hint explains the
# only part a user cannot guess: what "Same as Windows" follows.
settings-language-hint = Same as Windows follows your Windows display language. A choice here applies straight away and is remembered.
settings-theme = Theme
settings-startup = Start with Windows
settings-startup-hint = OmniToaster starts when you sign in. Moving the folder is fine — the setting follows it.
# Shown when the registry value points at a different copy of OmniToaster.
settings-startup-elsewhere = Another copy of OmniToaster is set to start with Windows, so this one is not.

## The settings status line. Sentence case; this is where non-modal truth
## lives, so each message says what is now true rather than what was clicked.

settings-status-ready = Ready.
# $time is the moment the pause lapses, formatted by ot_i18n.
settings-status-paused-until = Notifications are paused until { $time }.
settings-status-paused-until-resumed = Notifications are paused until you resume them.
settings-status-resumed = Notifications are on again.
settings-status-tray-failed = The tray icon could not be created, so closing this window is the only way out: { $reason }
settings-status-language-failed = The language could not be changed: { $reason }
# Status line after an upload. $label is the picture's name in the library.
settings-status-library-added = “{ $label }” was added to the picture library.
# Status line when the uploaded picture was already in the library. $label is its existing name.
settings-status-library-already-there = That picture is already in the library, as “{ $label }”.
# Status line after a picture was removed from the library.
settings-status-library-removed = The picture was removed from the library.
# Status line when the library could not be read or changed. $reason is the technical cause.
settings-status-library-failed = The picture library could not be updated: { $reason }
settings-status-dialog-failed = The file dialog could not be opened: { $reason }
settings-status-saved = Your settings are saved.
settings-status-autostart-on = OmniToaster will start with Windows.
settings-status-autostart-off = OmniToaster will not start with Windows.
# $reason is the underlying error, in English, from the operating system.
settings-status-save-failed = Your settings could not be saved: { $reason }
settings-status-autostart-failed = The startup setting could not be changed: { $reason }

## Theme names, shown in Settings ▸ General ▸ Theme, in the order the
## picker shows them. Dark is the default; System follows Windows.

settings-theme-system = System
settings-theme-dark = Dark
settings-theme-light = Light

## Date, time and number patterns. Only ot_i18n::format_datetime reads
## these; nothing else formats a date or a number
## (docs/10-i18n.md).

format-date-short = { $day }/{ $month }/{ $year }
format-time-24h = { $hour }:{ $minute }
format-time-12h = { $hour }:{ $minute } { $period }
format-period-am = am
format-period-pm = pm
# $date and $time are already formatted by the patterns above.
format-date-time = { $date } at { $time }

## Day names, used by format_datetime rather than by any OS locale, so a
## Malay day name is correct even when Windows is set to English.

day-monday = Monday
day-tuesday = Tuesday
day-wednesday = Wednesday
day-thursday = Thursday
day-friday = Friday
day-saturday = Saturday
day-sunday = Sunday

## Month names.

month-january = January
month-february = February
month-march = March
month-april = April
month-may = May
month-june = June
month-july = July
month-august = August
month-september = September
month-october = October
month-november = November
month-december = December
