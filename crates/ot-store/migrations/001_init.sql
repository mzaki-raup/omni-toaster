-- 001_init.sql — the Phase 0 schema (docs/05-data-model-and-storage.md).
--
-- A shipped migration is NEVER edited. A correction is a new migration, because
-- an edit here changes the schema of databases that have already applied it and
-- the runner has no way to notice.
--
-- 002_locale_keys.sql (Phase 4) adds the category key columns and
-- authored_lang; 003_jakim.sql (Phase 5) adds the zone table and the prayer
-- cache. Neither belongs here.

-- Mirror of settings.toml, for the values the scheduler must read inside a
-- transaction. settings.toml stays the source of truth (ADR-007).
CREATE TABLE settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
) STRICT;

-- Where the user is, for prayer times and weather. Coordinates are stored as
-- given; they are rounded before they are ever sent anywhere
-- (docs/06-security-and-privacy.md).
CREATE TABLE location (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    lat          REAL NOT NULL,
    lon          REAL NOT NULL,
    tz           TEXT NOT NULL,
    country_code TEXT,
    -- 'ip' | 'manual'. Which it was decides whether the app may re-detect.
    source       TEXT NOT NULL,
    updated_at   TEXT NOT NULL
) STRICT;

-- Imported media, content-addressed: the same image imported twice is stored
-- once. relative_path is relative to the data root so the folder can be moved.
CREATE TABLE asset (
    sha256        TEXT PRIMARY KEY NOT NULL,
    -- 'image' | 'audio' | 'icon'
    kind          TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    bytes         INTEGER NOT NULL,
    created_at    TEXT NOT NULL
) STRICT;

-- A notification template. One per Azan prayer, per weather condition and per
-- reminder type, inheriting from a global default
-- (docs/specs/toast.md section 8).
CREATE TABLE category (
    id             INTEGER PRIMARY KEY,
    -- 'prayer' | 'weather' | 'reminder'
    kind           TEXT NOT NULL,
    -- NULL means "use the built-in default text for the active language".
    -- Translated built-in text is never stored here.
    custom_title   TEXT,
    custom_body    TEXT,
    image_id       TEXT REFERENCES asset(sha256) ON DELETE SET NULL,
    sound_id       TEXT REFERENCES asset(sha256) ON DELETE SET NULL,
    accent_color   TEXT,
    style_json     TEXT,
    animation_json TEXT,
    enabled        INTEGER NOT NULL DEFAULT 1
) STRICT;

-- A thing to be reminded about. title and body are the user's own words and are
-- stored exactly as typed; the app never translates them.
CREATE TABLE reminder (
    id              INTEGER PRIMARY KEY,
    category_id     INTEGER REFERENCES category(id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    body            TEXT,
    -- 'at' | 'offset_before'
    trigger_type    TEXT NOT NULL,
    trigger_json    TEXT NOT NULL,
    recurrence_json TEXT,
    -- The one denormalised field: the scheduler reads this and nothing else to
    -- build its queue (docs/specs/reminders.md section 7).
    next_fire_at    TEXT,
    -- 'active' | 'snoozed' | 'done' | 'cancelled'
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
) STRICT;

-- The scheduler's hot path: "what is due next".
CREATE INDEX reminder_next_fire_at ON reminder (next_fire_at)
    WHERE next_fire_at IS NOT NULL;

CREATE INDEX reminder_status ON reminder (status);

-- Every snooze, with its running count, so the limit can be enforced and shown.
CREATE TABLE snooze_log (
    id            INTEGER PRIMARY KEY,
    reminder_id   INTEGER NOT NULL REFERENCES reminder(id) ON DELETE CASCADE,
    snoozed_until TEXT NOT NULL,
    count         INTEGER NOT NULL
) STRICT;

CREATE INDEX snooze_log_reminder ON snooze_log (reminder_id);

-- What actually fired, and what the user did about it. This is what makes
-- "did it fire?" answerable. Pruned at 90 days
-- (docs/05-data-model-and-storage.md).
CREATE TABLE notification_log (
    id          INTEGER PRIMARY KEY,
    reminder_id INTEGER REFERENCES reminder(id) ON DELETE SET NULL,
    fired_at    TEXT NOT NULL,
    -- 'shown' | 'snoozed' | 'dismissed' | 'opened' | 'missed'
    action      TEXT NOT NULL,
    -- Which source answered, for prayer times: 'api' | 'bundled' | 'calculated'
    source      TEXT
) STRICT;

CREATE INDEX notification_log_fired_at ON notification_log (fired_at);

-- Disposable: deleting it costs a refetch, never correctness.
CREATE TABLE weather_cache (
    location_id  INTEGER NOT NULL REFERENCES location(id) ON DELETE CASCADE,
    fetched_at   TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (location_id)
) STRICT;
