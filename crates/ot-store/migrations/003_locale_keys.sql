-- 003_locale_keys.sql — built-in keys and the language user text was written in
-- (FEAT-0039, docs/05-data-model-and-storage.md, docs/specs/reminders.md §9).
--
-- Planned as 002; Phase 2's 002_asset_label.sql took that number. Never edit
-- 001 or 002 to fold this in: a shipped migration is never edited.

-- A template names what it is for by a stable key, never by translated text:
-- 'prayer-fajr', 'weather-rain', 'reminder-fatigue-eye'. The key never changes
-- with the UI language; renaming one needs a migration. At most one of the
-- three is set, matching the row's kind.
ALTER TABLE category ADD COLUMN prayer_key TEXT;
ALTER TABLE category ADD COLUMN weather_key TEXT;
ALTER TABLE category ADD COLUMN type_key TEXT;

-- The language custom_title and custom_body were written in. NULL while they
-- are NULL, which means "the built-in text in the active language".
ALTER TABLE category ADD COLUMN authored_lang TEXT
    CHECK (authored_lang IN ('en-US', 'ms-MY'));

-- The language the user typed a reminder's title and body in. The app never
-- translates them; this is recorded so it can say which language they are in.
-- NULL only for rows from before this migration, of which there are none.
ALTER TABLE reminder ADD COLUMN authored_lang TEXT
    CHECK (authored_lang IN ('en-US', 'ms-MY'));

-- One template per key.
CREATE UNIQUE INDEX category_prayer_key ON category (prayer_key)
    WHERE prayer_key IS NOT NULL;
CREATE UNIQUE INDEX category_weather_key ON category (weather_key)
    WHERE weather_key IS NOT NULL;
CREATE UNIQUE INDEX category_type_key ON category (type_key)
    WHERE type_key IS NOT NULL;

-- The built-in reminder types (docs/specs/reminders.md §3), each a template
-- with no custom text, so it shows the Fluent default in whichever language is
-- active. Azan and weather changes are kinds of their own, seeded with their
-- keys by Phases 5 and 6.
INSERT INTO category (kind, type_key) VALUES
    ('reminder', 'reminder-event'),
    ('reminder', 'reminder-task'),
    ('reminder', 'reminder-anniversary'),
    ('reminder', 'reminder-fatigue-eye'),
    ('reminder', 'reminder-fatigue-posture'),
    ('reminder', 'reminder-fatigue-hydration'),
    ('reminder', 'reminder-fatigue-work-hours'),
    ('reminder', 'reminder-timer'),
    ('reminder', 'reminder-alarm'),
    ('reminder', 'reminder-payment'),
    ('reminder', 'reminder-appointment'),
    ('reminder', 'reminder-custom');
