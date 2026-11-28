-- 010_template_icon.sql — a template's own header icon (FEAT-0088,
-- docs/11-prayer-and-azan.md §8, docs/specs/toast.md §8.1).
--
-- A template could already name a picture (`image_id`) and a sound (`sound_id`) from
-- the libraries; this is the third: an icon for the toast's header. NULL is the app's
-- own icon. It references the same `asset` table, so removing an entry clears every
-- template's reference in the same statement, and "in use" can see it.
ALTER TABLE category ADD COLUMN icon_id TEXT REFERENCES asset(sha256) ON DELETE SET NULL;
