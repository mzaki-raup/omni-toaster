-- 004_jakim.sql — the prayer-times cache (FEAT-0053, docs/11-prayer-and-azan.md §4,
-- docs/05-data-model-and-storage.md).
--
-- There is no zone table: the 60 zones and their areas are embedded data
-- (data/jakim_zones.csv, FEAT-0050), not rows a user or a fetch could change.

-- One row per zone and day. The times are local clock values, 'HH:MM:SS' in
-- Malaysian time, exactly as e-Solat sends them: they become instants in
-- Asia/Kuala_Lumpur when the scheduler builds its queue, whatever the PC's zone.
CREATE TABLE jakim_prayer_cache (
    zone_code  TEXT NOT NULL,
    date       TEXT NOT NULL,           -- 'YYYY-MM-DD'
    hijri      TEXT NOT NULL,
    imsak      TEXT NOT NULL,
    fajr       TEXT NOT NULL,
    syuruk     TEXT NOT NULL,
    dhuha      TEXT NOT NULL,
    dhuhr      TEXT NOT NULL,
    asr        TEXT NOT NULL,
    maghrib    TEXT NOT NULL,
    isha       TEXT NOT NULL,
    fetched_at TEXT NOT NULL,           -- the instant the row was last written
    source     TEXT NOT NULL CHECK (source IN ('api', 'bundled')),
    PRIMARY KEY (zone_code, date)
);
