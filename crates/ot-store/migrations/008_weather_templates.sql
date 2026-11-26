-- 008_weather_templates.sql — a template for each weather condition (FEAT-0070,
-- docs/specs/weather.md §6, docs/specs/toast.md §8).
--
-- The column and its unique index (`category.weather_key`) have existed since
-- migration 003; only the rows are new. Seeded empty like the prayers': NULL
-- title, message, accent and picture mean the built-in text and picture in the
-- active language. For a weather template a NULL `sound_mode` is silence, not the
-- chime: a change of weather is not an Azan. The keys are the ones the settings
-- file uses (`clear`, `cloudy`, `rain`, `storm`, `snow`, `fog`) and never change
-- with the language.
INSERT INTO category (kind, weather_key) VALUES
    ('weather', 'clear'),
    ('weather', 'cloudy'),
    ('weather', 'rain'),
    ('weather', 'storm'),
    ('weather', 'snow'),
    ('weather', 'fog');
