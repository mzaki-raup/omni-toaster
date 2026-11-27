-- 009_weather_place.sql — the weather has its own place (FEAT-0072,
-- docs/specs/weather.md §9, docs/05-data-model-and-storage.md).
--
-- `location` keeps one row per use: id 1 is the place Prayer uses and id 2 the one
-- Weather uses. The table has no constraint on `id`, so this adds a row and no
-- column. A copy starts the weather place off where the prayer place is; from then on
-- they are independent. The cached weather belongs to the weather place, so its row
-- moves to id 2 (after the row it points at exists).
--
-- A fresh database has neither row, and nothing happens: the first prayer place saved
-- makes the weather's copy.
INSERT INTO location (id, name, lat, lon, tz, country_code, source, updated_at)
SELECT 2, name, lat, lon, tz, country_code, source, updated_at
FROM location
WHERE id = 1
  AND NOT EXISTS (SELECT 1 FROM location WHERE id = 2);

UPDATE weather_cache SET location_id = 2 WHERE location_id = 1
  AND EXISTS (SELECT 1 FROM location WHERE id = 2);
