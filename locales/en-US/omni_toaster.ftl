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
