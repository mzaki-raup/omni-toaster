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
# A button in the toast's action row: puts a reminder off for the length set in Settings ▸ Reminders. Greyed on a toast with nothing to snooze.
toast-action-snooze = Snooze
# A button in the toast's action row that closes the toast with its out-animation.
toast-action-dismiss = Dismiss
# A button in the toast's action row that brings Settings forward.
toast-action-open = Open
# Shown in the action row beside Dismiss, once a reminder has been snoozed as many times as Settings ▸ Reminders allows.
toast-snooze-limit = No more snoozes for this reminder.
# The title of the one toast that says reminders were missed while the PC was off or asleep.
toast-missed-title = Missed Reminders
# The body of that toast. "Open" is the toast's Open button, which shows the list.
toast-missed-body = Some reminders came due while the PC was off or asleep. Open shows them.

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

## Toast shapes, shown in Settings ▸ Appearance.

toast-shape-rectangle = Rectangle
toast-shape-rounded = Rounded Rectangle
toast-shape-pill = Pill

## Glass kinds, the Glass dropdown in Settings ▸ Appearance.

# No glass: the toast's background is solid.
toast-glass-none = None
# The desktop behind the toast, blurred, with almost none of the toast's colour.
toast-glass-blur = Blur
# The blurred desktop under the toast's colour, part-transparent.
toast-glass-acrylic = Acrylic
# The toast's colour, mostly opaque, with only a hint of the desktop through it.
toast-glass-mica-like = Mica-Like

## Toast animations, the Animation In and Animation Out dropdowns in
## Settings ▸ Animation. Title Case: they are list items.

# The toast slides in from beyond the screen's right edge.
toast-effect-slide-from-right = Slide from Right
# The toast slides up from beyond the bottom of the screen's working area.
toast-effect-slide-from-bottom = Slide from Bottom
# The toast pours out of a point at the bottom of the screen, narrow first, like a genie from a lamp.
toast-effect-genie-from-bottom = Genie from Bottom
# The toast grows from its centre, overshooting a little with spring easing.
toast-effect-pop-out = Pop Out
# The toast slides out of the screen edge it sits against, like a drawer.
toast-effect-drawer = Drawer
# The toast fades in or out. Also what every animation becomes when motion is reduced.
toast-effect-appear = Appear
# The toast builds up, or comes apart, grain by grain.
toast-effect-dissolve = Dissolve
# A halo in the toast's accent colour swells around it and settles.
toast-effect-glow = Glow
# The text is there first; the background fades in behind it.
toast-effect-alpha-to-solid = Alpha to Solid
# Out only: the toast pours into the tray icon, like Genie in reverse.
toast-effect-shrink-to-tray = Shrink to Tray

## How an animation's progress is shaped over its duration, the Easing dropdown.

# The same speed throughout.
toast-easing-linear = Linear
# Fast at first, settling gently at the end.
toast-easing-ease-out = Ease-Out
# Overshoots a little and settles, like a spring.
toast-easing-spring = Spring

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
# The section holding how toasts arrive and leave.
settings-section-animation = Animation
# The section holding the reminders and what Snooze does.
settings-section-reminders = Reminders

## Settings ▸ Reminders: the Snooze section. Title Case for the labels and
## list items, sentence case for the hints.

# A dropdown: how long one press of Snooze puts a reminder off.
settings-snooze-length = Snooze Length
# Hint under Snooze Length.
settings-snooze-length-hint = What one press of Snooze on a reminder's toast does.
# Snooze Length choice.
settings-snooze-5-minutes = 5 Minutes
# Snooze Length choice.
settings-snooze-15-minutes = 15 Minutes
# Snooze Length choice.
settings-snooze-1-hour = 1 Hour
# Snooze Length choice.
settings-snooze-1-day = 1 Day
# Snooze Length choice: a length the user types in.
settings-snooze-custom = Custom
# The field for a custom snooze length, shown when Custom is chosen. The unit is in the label, not after the number.
settings-snooze-custom-length = Custom Length (Minutes)
# A slider: how many times one reminder's toast may be snoozed.
settings-snooze-limit = Snooze Limit
# Hint under Snooze Limit.
settings-snooze-limit-hint = After this many snoozes, the toast offers only Dismiss.
# A slider: how many hours back a reminder missed while the PC was off or asleep is still listed. The unit is in the label.
settings-missed-window = Catch-Up Window (Hours)
# Hint under Catch-Up Window.
settings-missed-window-hint = Reminders missed while the PC was off or asleep are listed if they are this recent; older ones are dropped.
# The heading of the list of missed reminders, at the top of Settings ▸ Reminders.
settings-missed-heading = Missed Reminders
# The missed list's empty state.
settings-missed-empty = No reminders were missed.
# Under a missed reminder's title. $time is its date and time, already formatted.
settings-missed-due = Due { $time }

## Settings ▸ Animation. Title Case for the labels, sentence case for the hints.

# How a toast arrives.
settings-animation-enter = Animation In
# How a toast leaves.
settings-animation-leave = Animation Out
# How long each animation takes, in milliseconds.
settings-animation-duration = Duration
# How the animation's progress is shaped over its duration.
settings-animation-easing = Easing
# A checkbox: play every animation as a fade.
settings-animation-reduce = Reduce Motion
# Under Reduce Motion, when Windows' own animations setting is on.
settings-animation-windows-on = Windows shows animations, so the ones chosen here play unless Reduce Motion is ticked.
# Under Reduce Motion, when Windows' own animations setting is off.
settings-animation-windows-off = Windows' animations are turned off, so every animation plays as a fade.
# Under Reduce Motion, when it is ticked.
settings-animation-reduced = Every animation plays as a fade.
# Button: fires a test toast with the animations chosen above.
settings-animation-play = Play

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
# Under the Shadow checkbox when it is greyed out because the shape is not Rectangle.
settings-toast-shadow-hint = Windows draws a shadow only for the Rectangle shape.
# Under the Shadow checkbox when it is greyed out because glass is on.
settings-toast-shadow-glass-hint = Windows draws no shadow behind glass.
# Label of the dropdown choosing the glass kind.
settings-toast-glass = Glass
# Under the Glass dropdown when it is greyed out because the shape is not Rectangle.
settings-toast-glass-shape-hint = Glass is available for the Rectangle shape only: Windows blurs the whole square behind a toast, past rounded corners.
# Under the Glass dropdown when a glass kind is chosen: the preview cannot blur what is behind the settings window.
settings-toast-glass-hint = The preview shows the colour, not the blur. Show a test toast to see the blur.
settings-toast-background = Background
settings-toast-accent = Accent Colour
settings-toast-header = Header
settings-toast-header-icon = Icon
settings-toast-header-app-name = App Name
settings-toast-header-time = Time
settings-toast-position = Position
settings-toast-position-custom = Custom Point
# How long a toast stays on screen, in seconds (3 to 60).
settings-toast-duration = Display Duration
# The hint under that slider.
settings-toast-duration-hint = Pointing at a toast pauses its countdown until the pointer leaves.
# How many toasts may be on screen at once (1 to 5); the rest wait their turn.
settings-toast-max-visible = Visible at Once
# The space between one toast and the next in a stack, in pixels.
settings-toast-stack-gap = Gap Between Toasts
# The label of the picker choosing which monitor toasts appear on.
settings-toast-monitor = Monitor
# The hint under that picker.
settings-toast-monitor-hint = Toasts appear on this monitor. When it is not connected, they appear on the primary monitor.
# The first choice in the monitor picker, and the default: whichever monitor Windows calls primary.
settings-toast-monitor-primary = Primary Monitor
# A monitor in the picker. $number is its Windows display number, $name its own name, $width and $height its size in pixels.
settings-toast-monitor-option = { $number }: { $name } ({ $width } × { $height })
# The same, for the monitor that is currently the primary one.
settings-toast-monitor-option-primary = { $number }: { $name } ({ $width } × { $height }), Primary
# The name of a monitor that does not report one, as many laptop screens do. $number is its display number.
settings-toast-monitor-unnamed = Display { $number }
# The chosen monitor when it is not plugged in. $name is its name.
settings-toast-monitor-missing = { $name } (Not Connected)
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
# The heading of the list of pictures that ship with the app, in Settings > Picture Library.
settings-library-builtin = Built-In Pictures
# The line under that heading.
settings-library-builtin-hint = These come with OmniToaster. Use them for any notification; they can't be renamed or removed.
# Group headings in the picture pickers and the Picture Library: the built-in pictures for prayer times, weather and reminder types.
settings-builtin-group-prayer = Prayer Times
settings-builtin-group-weather = Weather
settings-builtin-group-reminder = Reminders
# Names of the built-in pictures, shown in the pickers and the Picture Library. Prayers carry both names, as everywhere in Settings.
settings-builtin-masjid-imsak = Mosque · Imsak
settings-builtin-masjid-subuh = Mosque · Subuh (Fajr)
settings-builtin-masjid-dhuha = Mosque · Dhuha
settings-builtin-masjid-zohor = Mosque · Zohor (Dhuhr)
settings-builtin-masjid-asar = Mosque · Asar (Asr)
settings-builtin-masjid-maghrib = Mosque · Maghrib
settings-builtin-masjid-isyak = Mosque · Isyak (Isha)
# Weather pictures.
settings-builtin-sunny = Sunny
settings-builtin-partly-cloudy = Partly Cloudy
settings-builtin-cloudy = Cloudy
settings-builtin-hazy = Hazy
settings-builtin-fog = Fog
settings-builtin-drizzle = Drizzle
settings-builtin-downpour = Downpour
settings-builtin-thunderstorm = Thunderstorm
settings-builtin-windy = Windy
# Reminder-type pictures.
settings-builtin-alarm-clock = Alarm Clock
settings-builtin-meeting = Meeting
settings-builtin-training = Training
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

## Settings ▸ General ▸ Clock: how times are written everywhere.

# The dropdown's label.
settings-clock = Clock
# The dropdown's hint.
settings-clock-hint = Automatic is 24-hour in Bahasa Melayu and follows Windows in English.
# A Clock choice.
settings-clock-automatic = Automatic
# A Clock choice.
settings-clock-12 = 12-Hour
# A Clock choice.
settings-clock-24 = 24-Hour

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
# The short date when Windows' regional setting puts the month first (m/d/yyyy).
format-date-short-month-first = { $month }/{ $day }/{ $year }
# An offset before a reminder's time. $count is a number.
format-before-minutes = { $count ->
    [one] 1 minute before
   *[other] { $count } minutes before
}
# An offset before a reminder's time. $count is a number.
format-before-hours = { $count ->
    [one] 1 hour before
   *[other] { $count } hours before
}
# An offset before a reminder's time. $count is a number.
format-before-days = { $count ->
    [one] 1 day before
   *[other] { $count } days before
}
# An offset before a reminder's time. $count is a number.
format-before-weeks = { $count ->
    [one] 1 week before
   *[other] { $count } weeks before
}
# How a reminder repeats. $count is a number. Lower case: it sits inside a sentence.
format-every-days = { $count ->
    [one] every day
   *[other] every { $count } days
}
# How a reminder repeats.
format-every-weeks = { $count ->
    [one] every week
   *[other] every { $count } weeks
}
# How a weekly reminder repeats. $days is a list of day names, already joined.
format-every-weeks-on = { $count ->
    [one] every week on { $days }
   *[other] every { $count } weeks on { $days }
}
# How a reminder repeats.
format-every-months = { $count ->
    [one] every month
   *[other] every { $count } months
}
# How a reminder repeats.
format-every-years = { $count ->
    [one] every year
   *[other] every { $count } years
}
# Two items of a list: "Monday and Thursday".
format-list-pair = { $first } and { $second }
# Three or more: the first, then the rest already joined — "Monday, Tuesday and Thursday".
format-list-more = { $first }, { $rest }

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

## Reminder types. Each key is also the stable identifier stored in the
## database (category.type_key), so a key is never renamed. The name is a list
## item, so Title Case. The -body line is the toast's text when neither the
## reminder nor its template has a body of its own: sentence case. A user's
## own title and body are never keys and are never translated.

# The type of a reminder for a dated event: a talk, a course, a gathering.
reminder-event = Event
# Toast body for an Event reminder that has no text of its own.
reminder-event-body = An event is about to start.
# The type of a reminder for something the user has to do.
reminder-task = Task
# Toast body for a Task reminder that has no text of its own.
reminder-task-body = A task is due.
# The type of a reminder for a yearly date: a birthday, a wedding anniversary.
reminder-anniversary = Anniversary
# Toast body for an Anniversary reminder that has no text of its own.
reminder-anniversary-body = Remember this anniversary.
# Fatigue reminder: rest the eyes from the screen.
reminder-fatigue-eye = Eye Break
# Toast body for an Eye Break reminder that has no text of its own.
reminder-fatigue-eye-body = Rest your eyes: look at something far away.
# Fatigue reminder: check how you are sitting.
reminder-fatigue-posture = Posture
# Toast body for a Posture reminder that has no text of its own.
reminder-fatigue-posture-body = Sit up straight and relax your shoulders.
# Fatigue reminder: drink water.
reminder-fatigue-hydration = Drink Water
# Toast body for a Drink Water reminder that has no text of its own.
reminder-fatigue-hydration-body = Time for a glass of water.
# Fatigue reminder: the working day has gone on long enough.
reminder-fatigue-work-hours = Work-Hour Limit
# Toast body for a Work-Hour Limit reminder that has no text of its own.
reminder-fatigue-work-hours-body = You have reached your work-hour limit.
# The type of a reminder for a countdown.
reminder-timer = Timer
# Toast body for a Timer reminder that has no text of its own.
reminder-timer-body = Your timer has finished.
# The type of a reminder that wakes or alerts: the most urgent kind.
reminder-alarm = Alarm
# Toast body for an Alarm reminder that has no text of its own.
reminder-alarm-body = Your alarm is going off.
# The type of a reminder for a bill or other payment.
reminder-payment = Payment
# Toast body for a Payment reminder that has no text of its own.
reminder-payment-body = A payment is due.
# The type of a reminder for a meeting with someone: a doctor, a client.
reminder-appointment = Appointment
# Toast body for an Appointment reminder that has no text of its own.
reminder-appointment-body = You have an appointment.
# The type of a reminder that fits none of the others.
reminder-custom = Custom
# Toast body for a Custom reminder that has no text of its own.
reminder-custom-body = This is your reminder.

## Settings ▸ Reminders: the list and the editor. Title Case for labels,
## buttons and list items; sentence case for hints, empty states, questions
## and validation messages.

# The heading above the list of reminders.
settings-reminders-heading = Your Reminders
# The button that starts a new reminder in the editor.
settings-reminders-new = New Reminder
# Shown while the list is being read.
settings-reminders-loading = Loading your reminders…
# The list's empty state.
settings-reminders-empty = You have no reminders yet. New Reminder makes one.
# A list row's second line. $type is the reminder's type; $time when it next fires, already formatted.
settings-reminder-row-next = { $type } · next { $time }
# A list row's second line for a snoozed reminder. $time is when the snooze ends, already formatted.
settings-reminder-row-snoozed = { $type } · snoozed until { $time }
# A list row's second line for a reminder that will not fire again.
settings-reminder-row-done = { $type } · done
# A list row's second line while its next time is being worked out.
settings-reminder-row-plain = { $type }
# A repeating reminder's second line. $repeat is how it repeats ("every 2 weeks on Monday"); $time when it next fires.
settings-reminder-row-next-repeat = { $type } · { $repeat } · next { $time }
# The sentence under Repeat in the editor. $repeat is how it repeats.
settings-reminder-repeat-summary = Repeats { $repeat }.
# The heading of the fatigue section in Settings ▸ Reminders: eye break, posture, drink water, work-hour limit.
settings-fatigue-heading = Fatigue Reminders
# After an eye break's, posture's or drink's number. $count is that number.
settings-fatigue-minutes-apart = { $count ->
    [one] minute apart
   *[other] minutes apart
}
# After the work-hour limit's number. $count is that number.
settings-fatigue-hours-of-work = { $count ->
    [one] hour of work
   *[other] hours of work
}
# The hint under the fatigue section.
settings-fatigue-hint = Each one counts from when you start using the PC. An hour or more away from it starts the day again.
# Editor field: the reminder's own title.
settings-reminder-title = Title
# Editor field: the reminder's own text, shown in its toast.
settings-reminder-body = Details
# Editor field: the reminder's type.
settings-reminder-type = Type
# Editor field: the day it is for.
settings-reminder-date = Date
# Editor field: the time of day.
settings-reminder-time = Time
# Editor field: toasts ahead of the time itself.
settings-reminder-offsets = Remind Me Before
# Adds a row to Remind Me Before.
settings-reminder-offset-add = Add a Time Before
# Removes one row of Remind Me Before.
settings-reminder-offset-remove = Remove
# Hint under Remind Me Before.
settings-reminder-offsets-hint = Each one is a toast of its own, as well as the one at the time itself.
# A unit, in the Remind Me Before and Repeat dropdowns.
settings-unit-minutes = Minutes
# A unit.
settings-unit-hours = Hours
# A unit.
settings-unit-days = Days
# A unit.
settings-unit-weeks = Weeks
# A unit.
settings-unit-months = Months
# A unit.
settings-unit-years = Years
# Editor field: how the reminder repeats.
settings-reminder-repeat = Repeat
# Repeat choice: it fires once.
settings-repeat-never = Never
# Editor field: every how many of the Repeat unit — "Repeat Every 2" with Weeks is every two weeks.
settings-reminder-every = Repeat Every
# Editor field: the weekdays of a weekly repeat.
settings-reminder-weekdays = On
# Hint under the weekdays.
settings-reminder-weekdays-hint = With none ticked, it repeats on the date's own weekday.
# Editor button.
settings-reminder-save = Save
# Editor button: leave without saving.
settings-reminder-cancel = Cancel
# Editor button, and the confirming button of the Delete question.
settings-reminder-delete = Delete
# The button that backs out of the Delete or Clear History question.
settings-reminder-keep = Keep It
# The question Delete asks first.
settings-reminder-delete-confirm = Delete this reminder? Its history stays until it ages out.
# The heading of the edited reminder's history.
settings-reminder-history = History
# The history's empty state.
settings-reminder-history-empty = Nothing has happened to this reminder yet.
# The button that clears the edited reminder's history, and the confirming button of its question.
settings-reminder-history-clear = Clear History
# The question Clear History asks first.
settings-reminder-history-clear-confirm = Clear this reminder's history? This cannot be undone.
# One history line. $time is when, already formatted; $action what happened.
settings-reminder-history-row = { $time } — { $action }
# What happened to a fire, in the history.
settings-history-shown = Shown
# What happened to a fire.
settings-history-snoozed = Snoozed
# What happened to a fire.
settings-history-dismissed = Dismissed
# What happened to a fire.
settings-history-opened = Opened
# What happened to a fire.
settings-history-missed = Missed
# Said under Title when it is empty.
error-reminder-title-empty = Give the reminder a title.
# Said under Time when every fire would be in the past.
error-reminder-time-passed = This time has passed and the reminder does not repeat, so it would never fire.
# The status line when the reminders could not be read or saved. $reason is the cause.
settings-status-reminders-failed = Your reminders could not be read or saved: { $reason }
# The prayer source label, always shown beside the day's times (Settings > Prayer, the tray tooltip). Times fetched from JAKIM e-Solat. $time is when they were fetched, formatted by the shared formatter.
prayer-source-jakim = Source: JAKIM e-Solat, fetched { $time }
# The same label when the times come from the saved copy because a fetch was not possible or not needed. $time is when that copy was fetched.
prayer-source-jakim-cached = Source: JAKIM e-Solat (saved copy), fetched { $time }
# The prayer source label when the times come from the dataset shipped inside the app.
prayer-source-bundled = Offline data
# The prayer source label when the times are calculated because the user chose that, or the location is outside Malaysia.
prayer-source-calculated = Calculated
# The same label when the location is inside Malaysia, where JAKIM's own times can differ by a few minutes.
prayer-source-calculated-approximate = Calculated (approximate)
# The prayer source label when JAKIM times were wanted but could not be had, so the app calculated them instead.
prayer-source-calculated-fallback = Calculated (fallback)
# The same fallback label when the location is inside Malaysia, where the calculation can differ from JAKIM by a few minutes.
prayer-source-calculated-fallback-approximate = Calculated (fallback, approximate)
# The name of the Prayer section in the settings window's left-hand list.
settings-section-prayer = Prayer
# The name of the Weather section in the settings window's left-hand list.
settings-section-weather = Weather
# The name of the Updates section in the settings window's left-hand list.
settings-section-updates = Updates
# The heading of the location group in Settings > Prayer.
prayer-location-heading = Location
# The question in the first-run card that asks before the app looks up the user's place from their internet address.
prayer-consent-title = Detect your location from your internet address?
# The explanation under that question. ipapi.co is the name of the lookup service and stays as written.
prayer-consent-body = OmniToaster can look up where you are once a day, to choose your prayer zone. Your internet address is sent to ipapi.co and is not kept. You can type a place instead.
# The button that agrees to the lookup.
prayer-consent-allow = Allow
# The button that declines the lookup for now; the user can still change their mind later.
prayer-consent-decline = Not Now
# The checkbox that turns the daily lookup on or off after the first answer.
prayer-detect-checkbox = Detect my location from my internet address
# The hint under that checkbox.
prayer-detect-hint = Once a day, only with your permission. A place you set yourself is never replaced.
# Shown while the lookup is running.
prayer-location-looking = Looking up your location…
# Shown when the lookup could not reach the service.
prayer-error-network = The location service could not be reached. Check your connection, or type a place instead.
# Shown when the location service refused the request, for example because it was asked too often.
prayer-error-service = The location service did not give an answer. Try again later, or type a place instead.
# Shown when the location service answered with something that could not be used.
prayer-error-answer = The location service gave an answer that could not be used. Type a place instead.
# The saved place when it was found from the internet address. $place is a city and region.
prayer-location-detected = Location: { $place } (detected from your internet address)
# The saved place when the user chose it. $place is a city and region, or coordinates.
prayer-location-manual = Location: { $place } (set by you)
# Shown when there is no saved place yet.
prayer-location-none = No location is set. Search for a city or type coordinates.
# The button that opens the place entry controls.
prayer-location-change = Change Location
# The button that closes the place entry controls.
prayer-close = Close
# The label of the city search field.
prayer-search-label = City
# The grey hint inside the empty city search field.
prayer-search-placeholder = Search for a city
# The button that runs the city search.
prayer-search-button = Search
# Shown while a city search is running.
prayer-search-loading = Searching…
# Shown when a city search found nothing.
prayer-search-empty = No city matches that search.
# One city search result. $place is a city and region; $zone is its time zone name.
prayer-search-result = { $place } ({ $zone })
# The label of the coordinates field.
prayer-coords-label = Coordinates
# The grey hint inside the empty coordinates field.
prayer-coords-placeholder = Latitude, longitude, for example 3.14, 101.69
# The button that uses the typed coordinates.
prayer-coords-use = Use Coordinates
# Shown when the typed coordinates are not two numbers.
prayer-coords-error-format = Type a latitude and a longitude, like 3.14, 101.69.
# Shown when the typed coordinates are outside the Earth's range.
prayer-coords-error-range = Latitude is from -90 to 90 and longitude from -180 to 180.
# Shown when this PC's own time zone could not be read for coordinates outside Malaysia.
prayer-coords-error-zone = This PC's time zone could not be read. Search for a city instead.
# Shown after coordinates outside Malaysia are used. $zone is this PC's time zone, which was assumed.
prayer-coords-zone-note = Time zone: { $zone }, taken from this PC. Check that it is right for this place, or search for a city instead.
# The label of the source dropdown.
prayer-source-label = Source
# The first choice in the source dropdown: JAKIM inside Malaysia and calculated elsewhere.
prayer-choice-auto = Auto
# The second choice in the source dropdown.
prayer-choice-jakim = JAKIM e-Solat
# The third choice in the source dropdown.
prayer-choice-calculated = Calculated
# The hint under the source dropdown.
prayer-source-hint = Auto uses JAKIM e-Solat inside Malaysia and calculates elsewhere.
# A warning shown when JAKIM is chosen but there is no zone to ask about.
prayer-warning-needs-zone = JAKIM needs a zone. Choose one below, or use Auto.
# A warning shown when JAKIM is chosen for a place outside Malaysia.
prayer-warning-abroad = This place is outside Malaysia. JAKIM times follow the zone you chose and may differ from local custom.
# The label of the zone row.
prayer-zone-label = Zone
# The zone found from the saved place. $code is a JAKIM zone code like SGR01; $areas is a list of areas; $state is a state.
prayer-zone-detected = Detected zone: { $code } · { $areas } · { $state }
# The zone the user chose. The same placeholders as the detected zone.
prayer-zone-chosen = Zone: { $code } · { $areas } · { $state }
# Shown when there is no zone yet.
prayer-zone-none = No zone yet. Set a location in Malaysia, or choose a zone.
# The button that opens the zone list.
prayer-zone-change = Change Zone
# The button that goes back to the zone found from the place.
prayer-zone-use-detected = Use Detected Zone
# The grey hint inside the empty zone search field.
prayer-zone-search-placeholder = Search by code, state or area
# Shown when a zone search found nothing.
prayer-zone-no-match = No zone matches that search.
# One row of the zone list. $code is a zone code, $state a state, $areas a list of areas.
prayer-zone-row = { $code } · { $state } · { $areas }
# The label of the calculation method dropdown.
prayer-method-label = Method
# A calculation method.
prayer-preset-mwl = Muslim World League
# A calculation method.
prayer-preset-isna = ISNA
# A calculation method.
prayer-preset-umm-al-qura = Umm al-Qura
# A calculation method that follows JAKIM's angles but is not JAKIM's own times.
prayer-preset-malaysia = Malaysia (JAKIM Angles, Approximate)
# The label of the Asar school dropdown. Asar is the afternoon prayer.
prayer-asr-label = Asar School
# An Asar school; the one JAKIM follows.
prayer-asr-shafii = Shafi'i
# An Asar school.
prayer-asr-hanafi = Hanafi
# The hint under the Asar school dropdown.
prayer-asr-hint = Shafi'i is JAKIM's convention. Hanafi puts Asar about an hour later.
# The heading of the list of today's prayer times.
prayer-today-heading = Today's Prayer Times
# Shown while today's times are being worked out.
prayer-today-loading = Working out today's times…
# Shown when no source could give today's times.
prayer-today-nothing = No prayer times are available. Set a location, or check your connection and try again.
# Shown before there is a location to work from.
prayer-today-none-yet = Choose a location to see today's times.
# A prayer in the day's list: both names, so a reader of either can find it. Fajr is the dawn prayer.
prayer-name-fajr = Fajr (Subuh)
# Sunrise, which ends the dawn prayer's time.
prayer-name-syuruk = Sunrise (Syuruk)
# The midday prayer.
prayer-name-dhuhr = Dhuhr (Zohor)
# The afternoon prayer.
prayer-name-asr = Asr (Asar)
# The sunset prayer.
prayer-name-maghrib = Maghrib
# The name of the Friday prayer in English, with the Malay name beside it, in lists of prayers and in its notification.
prayer-name-jumuah = Jumu'ah (Jumaat)
# The night prayer.
prayer-name-isha = Isha (Isyak)
# The heading of the group of notification settings in Settings > Prayer.
prayer-notify-heading = Notifications
# The checkbox that gives Syuruk (sunrise) its own notification. The five daily prayers always have one.
prayer-notify-syuruk = Notify at Sunrise (Syuruk)
# The checkbox that gives Jumu'ah, the Friday prayer, a notification. On Fridays it takes the place of Dhuhr's, at the same time.
prayer-notify-jumuah = Notify for Jumu'ah (Replaces Dhuhr on Fridays)
# The hint under the Syuruk checkbox.
prayer-notify-hint = The five daily prayers always have a notification.
# The label of the number field for how old a missed prayer may be and still be listed.
prayer-azan-window-label = Missed Prayer Window (Minutes)
# The hint under that field.
prayer-azan-window-hint = A prayer missed while the PC was off is listed if it is no older than this. Older ones are dropped.
# The label of the field for how many minutes before a prayer its reminder comes.
prayer-before-label = Remind Before (Minutes)
# The hint under the per-prayer "remind before" checkboxes.
prayer-before-hint = Tick a prayer to get a short reminder this many minutes ahead of it. A reminder that would come after its prayer has started is skipped.
# The body of a prayer's notification. $prayer is the prayer's name with both names, like Fajr (Subuh).
prayer-toast-body = It is time for { $prayer }.
# The body of the reminder ahead of a prayer. $prayer is the prayer's name with both names, like Fajr (Subuh). $count is the number of minutes.
prayer-before-body = { $count ->
    [one] { $prayer } is in 1 minute.
   *[other] { $prayer } is in { $count } minutes.
}
# The title of the notification shown, at most once a day, when no prayer times could be worked out.
prayer-toast-unavailable-title = No prayer times available
# The body of that notification.
prayer-toast-unavailable-body = OmniToaster could not work out today's prayer times. Open Settings and choose a location.
# The heading of the group where each prayer's notification can be changed.
prayer-template-heading = Prayer Templates
# The label of the dropdown that chooses which prayer's template is edited.
prayer-template-prayer = Prayer
# The label of the title field of a prayer's notification.
prayer-template-title = Title
# The label of the message field of a prayer's notification.
prayer-template-message = Message
# The checkbox that gives a prayer's notification an accent colour of its own.
prayer-template-accent-use = Use a custom accent colour
# The label of the picture dropdown of a prayer's notification.
prayer-template-picture = Picture
# The first choice in that dropdown: the picture that ships with the app for this prayer.
prayer-template-picture-builtin = Built-in picture
# The hint under the template fields.
prayer-template-hint = Leave a field empty to use the built-in text in the current language. Text you write is kept exactly as typed and is never translated.
# The button that saves the template.
prayer-template-save = Save Template
# The button that clears the title and message so the built-in text is used again.
prayer-template-default = Use Default Text
# The button that shows the notification as it would look.
prayer-template-preview = Preview Toast
# Shown after the template was saved.
prayer-template-saved = Template saved.
# The label of the dropdown that chooses what a prayer plays when it is time.
prayer-sound-label = Sound
# A sound choice: the short chime that ships with the app. The default.
prayer-sound-chime = Short Chime
# A sound choice: the full Azan that ships with the app.
prayer-sound-azan = Full Azan
# A sound choice: an audio clip the user picked from their own files.
prayer-sound-file = My Audio File
# A sound choice: no sound at all.
prayer-sound-none = Silent
# The button that opens the dialog for choosing an audio clip.
prayer-sound-choose = Choose Audio File
# Shown while no audio clip has been chosen yet.
prayer-sound-none-chosen = No file chosen yet.
# The button that plays the prayer's sound now, to hear it.
prayer-sound-play = Play Sound
# The button that stops the sound.
prayer-sound-stop = Stop
# The title of the dialog for choosing an audio clip.
prayer-sound-dialog-title = Choose an audio clip
# The name of the file type filter in that dialog.
prayer-sound-filter = Audio (WAV, MP3, Ogg)
# Said when an audio clip is over the size limit.
error-import-audio-too-large = That clip is over 20 MB.
# Said when a chosen file is not an audio file of a kind the app plays, whatever its name says.
error-import-not-audio = That file is not a WAV, MP3 or Ogg Vorbis clip, whatever its name says.
# Said when an audio clip runs longer than ten minutes.
error-import-audio-too-long = That clip runs longer than ten minutes.
# The name of a weather condition: clear sky or mostly clear. Shown in Settings and in the weather notification.
weather-clear = Clear
# The name of a weather condition: partly cloudy or overcast.
weather-cloudy = Cloudy
# The name of a weather condition: drizzle, rain or rain showers.
weather-rain = Rain
# The name of a weather condition: a thunderstorm.
weather-storm = Thunderstorm
# The name of a weather condition: snow, snow grains or snow showers.
weather-snow = Snow
# The name of a weather condition: fog.
weather-fog = Fog
# The name shown when the weather service sends a condition this app does not know. Never an English word from the service.
weather-unknown = Unclear conditions
# The body of the notification when the weather changes. $from and $to are condition names (weather-rain and so on).
weather-toast-condition-body = The weather changed from { $from } to { $to }.
# The title of the notification when the temperature falls quickly.
weather-toast-fall-title = Temperature is falling
# The body of that notification. $degrees is how many whole degrees Celsius it fell, $now the temperature now, and $count the number of hours it fell over.
weather-toast-fall-body = { $count ->
    [one] It is { $degrees } °C colder than an hour ago, now { $now } °C.
   *[other] It is { $degrees } °C colder than { $count } hours ago, now { $now } °C.
}
# The title of the notification when the temperature rises quickly.
weather-toast-rise-title = Temperature is rising
# The body of that notification. $degrees is how many whole degrees Celsius it rose, $now the temperature now, and $count the number of hours it rose over.
weather-toast-rise-body = { $count ->
    [one] It is { $degrees } °C warmer than an hour ago, now { $now } °C.
   *[other] It is { $degrees } °C warmer than { $count } hours ago, now { $now } °C.
}
# The heading of Settings > Weather.
weather-heading = Weather Notifications
# The checkbox that turns weather notifications on. Off by default.
weather-enable = Notify me when the weather changes
# The hint under that checkbox: what is sent, and that off sends nothing.
weather-enable-hint = Sends the place saved under Prayer, rounded to about a kilometre, to Open-Meteo. Turned off, nothing is sent.
# The label of the row that names the place the weather is for.
weather-place-label = Place
# Shown when no place has been saved yet.
weather-place-none = No place is set. Set one under Prayer.
# The hint under the place.
weather-place-hint = The weather uses the same place as the prayer times.
# The label of the field for how many minutes pass between looks at the weather.
weather-poll-label = Check Every (Minutes)
# The hint under that field.
weather-poll-hint = The service updates about every 15 minutes, so asking more often changes nothing.
# The heading of the list of kinds of weather to be told about.
weather-notify-heading = Tell Me When It Becomes
# The hint under that list.
weather-notify-hint = A change is told only once the new weather has been seen twice in a row, so weather that flickers never notifies.
# The heading of the sudden temperature change settings.
weather-temperature-heading = Sudden Temperature Changes
# The checkbox for telling about a quick fall in temperature.
weather-fall-enable = Tell me when the temperature falls quickly
# The checkbox for telling about a quick rise in temperature.
weather-rise-enable = Tell me when the temperature rises quickly
# The label of the field for how many degrees Celsius the temperature must move.
weather-delta-label = By at Least (°C)
# The label of the field for the number of hours the move must happen within.
weather-window-label = Within (Hours)
# The hint under the temperature settings.
weather-temperature-hint = For example 5 °C in 3 hours. It is told once, and again only after the temperature has settled.
# The heading of the weather templates.
weather-template-heading = Weather Templates
# The label of the list for choosing which condition's template to edit.
weather-template-condition = Condition
# Said when the weather settings or a template could not be read or saved. The cause is in the log.
weather-error = The weather settings could not be read or saved.
# The credit line at the bottom of Settings > Weather. Open-Meteo's data is licensed CC BY 4.0, which asks for credit.
weather-credit = Weather data by Open-Meteo.com, licensed CC BY 4.0.
# The heading of the block showing the weather as it stands.
weather-now-heading = Current Weather
# The label of the row giving the weather now.
weather-now-label = Now
# The weather as it stands. $condition is a condition name (weather-rain and so on), $temperature whole degrees Celsius.
weather-now = { $condition }, { $temperature } °C
# Under the weather when it is up to date. $age is how long ago, like "5 minutes ago" or "just now".
weather-updated = Updated { $age }.
# The marker shown instead when the weather is out of date, for instance when offline. $age is how long ago it was fetched, like "3 hours ago". No change is ever reported from such data.
weather-stale = Not up to date: showing the weather from { $age }.
# Shown when weather is on but nothing has been found yet.
weather-now-none = No weather has been found yet.
# Shown when weather notifications are off.
weather-now-off = Weather notifications are off.
# How long ago, for a moment less than a minute ago.
weather-age-now = just now
# How long ago, in minutes. $count is a number.
weather-age-minutes = { $count ->
    [one] 1 minute ago
   *[other] { $count } minutes ago
}
# How long ago, in hours. $count is a number.
weather-age-hours = { $count ->
    [one] 1 hour ago
   *[other] { $count } hours ago
}
# How long ago, in days. $count is a number.
weather-age-days = { $count ->
    [one] 1 day ago
   *[other] { $count } days ago
}

# The heading of Settings > Updates.
update-heading = Updates
# Label of the row that shows the version of the program that is running.
update-version-label = Version
# Label of the row that shows which website this copy looks for updates on.
update-source-label = Updates from
# Hint under the source: it was fixed when this copy was built, so it is shown and cannot be edited.
update-source-hint = Chosen when this copy was built. It can't be changed here.
# Checkbox that turns checking for updates on or off. Off sends nothing.
update-enabled = Check for updates
# Hint under that checkbox: what turning it off does.
update-enabled-hint = When off, this copy sends nothing to look for updates, and Check Now does nothing.
# Checkbox that makes the app look for updates on its own, once a day at most.
update-automatic = Check automatically once a day
# Hint under that checkbox: it only looks, and never installs by itself.
update-automatic-hint = Looks for a newer version at most once a day while the app is running. It never installs anything without asking.
# Button that looks for a newer version right now.
update-check-now = Check Now
# Button that stops a check or a download that is under way.
update-cancel = Cancel
# Shown while the app asks the website whether there is a newer version.
update-checking = Checking for updates…
# Shown when the newest version is not newer than this one. $version is this copy's version.
update-up-to-date = You have the latest version ({ $version }).
# Shown when a newer version can be installed. $version is that version.
update-available = Version { $version } is available.
# Shown when a newer version exists but cannot be installed from inside the app. $version is that version.
update-not-installable = Version { $version } is out, but it can't be installed from here. Download it from its release page.
# Button that downloads, checks and prepares the newer version.
update-install = Download and Install
# Shown while the new version downloads. $version is that version and $percent a whole number from 0 to 100.
update-downloading = Downloading version { $version }: { $percent }%
# Shown when the new version is ready and waiting for a restart. $version is that version.
update-staged = Version { $version } is ready. Restart to finish installing it.
# Button that restarts the app into the new version.
update-restart = Restart to Finish
# Link to the page of the newer version, where its release notes are.
update-release-notes = Release Notes
# Shown when the last check is known. $age is how long ago, such as "3 hours ago".
update-last-checked = Last checked { $age }.
# Shown when nothing has been checked yet.
update-never-checked = Not checked yet.
# Shown when checking for updates is turned off.
update-off = Checking for updates is turned off.
# Error: the website could not be reached. $source is its address.
update-failed-network = The update check could not reach { $source }. Try again later.
# Error: the website has no published version at all.
update-failed-none = No version has been published yet.
# Error: the signature or the manifest of the update was not acceptable.
update-failed-rejected = The update was rejected because its signature or details did not check out. Nothing was installed.
# Error: the downloaded file is not the one that was signed.
update-failed-download = The download did not match what was signed, so it was deleted. Nothing was installed.
# Error: the folder the app is in cannot be written to.
update-failed-folder = The folder this app is in can't be written to, so it can't update itself. Download the zip from the release page instead.
# Error: this copy is too old to update in place. $version is the oldest that can.
update-failed-too-old = This copy is too old to update in place. Download version { $version } or newer from the release page.
# Error: anything else. The log has the detail.
update-failed-other = The update could not be completed. The log has the details.
# Shown after an update did not start properly and the old version was put back.
update-rolled-back = The last update did not start properly, so the previous version was put back.
