-- 006_prayer_sound.sql — which sound a prayer plays (FEAT-0060,
-- docs/11-prayer-and-azan.md §9).
--
-- NULL is the short chime, the default. 'azan' is the full Azan that ships in the
-- exe, 'file' is a clip the user imported (named by the existing `sound_id`, an
-- asset), and 'none' is silent.
ALTER TABLE category ADD COLUMN sound_mode TEXT
    CHECK (sound_mode IN ('chime', 'azan', 'file', 'none'));
