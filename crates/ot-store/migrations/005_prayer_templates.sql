-- 005_prayer_templates.sql — a template for each prayer (FEAT-0059,
-- docs/specs/toast.md §8, docs/11-prayer-and-azan.md §9).
--
-- Seeded empty: `custom_title`, `custom_body`, `accent_color` and `image_id` are
-- NULL, which means "the built-in text and picture, in the active language".
-- Only what the user types is stored. The key is the scheduler's, and never
-- changes with the language; renaming one would need a migration.
INSERT INTO category (kind, prayer_key) VALUES
    ('prayer', 'fajr'),
    ('prayer', 'syuruk'),
    ('prayer', 'dhuhr'),
    ('prayer', 'asr'),
    ('prayer', 'maghrib'),
    ('prayer', 'isha');
