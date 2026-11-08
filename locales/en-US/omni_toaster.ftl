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
# $path is the crash report folder.
error-crash-loop = OmniToaster stopped restarting after repeated crashes. The crash reports are in { $path }.

## Language names, shown in Settings ▸ General ▸ Language. Each name is
## written in its own language, which is what users expect in a language
## picker.

language-system = Same as Windows
language-en-us = English
language-ms-my = Bahasa Melayu

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
