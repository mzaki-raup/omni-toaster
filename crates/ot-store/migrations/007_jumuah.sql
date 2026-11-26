-- 007_jumuah.sql — a template for Jumu'ah, the Friday prayer (docs/11-prayer-and-azan.md §9).
--
-- Seeded empty like the other prayers': NULL means the built-in text and
-- picture in the active language. Jumu'ah is kept at Dhuhr's time on Fridays and
-- replaces Dhuhr's notification that day; the key is the scheduler's and never
-- changes with the language.
INSERT INTO category (kind, prayer_key) VALUES ('prayer', 'jumuah');
