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
settings-language = Language
# Under the language picker. The window re-renders in the new language only
# after a restart until the live switch lands.
settings-language-hint = The window changes language the next time OmniToaster starts.
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
