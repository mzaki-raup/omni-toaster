-- 002_asset_label.sql — the image library (FEAT-0025, docs/specs/toast.md 8.1).
--
-- The name the user gives a picture in the library ("Mosque", "Meeting"), so a
-- picker reads as names with thumbnails rather than hashes. Empty for an asset
-- that is not in the library, which is what every row before this was.
ALTER TABLE asset ADD COLUMN label TEXT NOT NULL DEFAULT '';
