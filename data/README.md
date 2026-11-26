# Data files

Embedded in the exe. Each changes only through a reviewed commit, with its
source noted in `docs/CHANGELOG.md` (`docs/11-prayer-and-azan.md` §3).

## `jakim_zones.csv` — the JAKIM prayer-time zones

One row per zone: `code`, `state_ms`, `state_en`, `areas_ms`, `areas_en`.

- **Source:** JAKIM's e-Solat portal, the zone list on
  `https://www.e-solat.gov.my/index.php?siteId=24&pageId=24`, read
  2026-11-20 — 60 zones.
- `state_ms` and `areas_ms` are exactly as JAKIM publishes them, spellings and
  spacing included (`S.Alam`, `Sg. Siput ,`, `Genting Higlands`).
- `state_en` and `areas_en` keep every place name and translate only the
  joining words and labels: *dan* → *and*, *Seluruh Negeri* → *All of*,
  *Bahagian X (Timur)* → *X Division (East)*, *Daerah Kecil* → *Sub-district*,
  *Bandar* → *Town*, *Zon Khas* → *Special Zone*; *Pulau Pinang* → *Penang*,
  *Wilayah Persekutuan* → *Federal Territories*; and JAKIM's abbreviations and
  one typo written out (*Shah Alam*, *Sungai Siput*, *Kampung Gajah*, *Genting
  Highlands*).

## `jakim_zone_points.csv` — where each zone's areas are

One row per point: `code`, `place`, `latitude`, `longitude`, `geonameid`. The
nearest point to a location gives its zone; a zone has a point for each of its
listed areas that could be placed, 208 in all, so the zones that meet across
the Klang Valley are told apart by their towns rather than by one centre each.

- **Source:** [GeoNames](https://www.geonames.org/), the Malaysia extract
  `MY.zip` from `download.geonames.org/export/dump/`, downloaded 2026-11-20
  (SHA-256 `1e93c290…bd25b4`), licensed under
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Coordinates are
  rounded to four decimal places (about 11 m).
- Each area was matched by name within its own state, preferring a town with a
  population, then a district, then an island, peak or pass. Chosen by hand by
  GeoNames id: Mukim Chiku (Kemajuan Tanah Chiku Satu), Jengka (Kemajuan Tanah
  Jengka Satu), Genting Sempah (the pass), Sahabat (the Sahabat airstrip) and
  Bukit Tinggi (Kampung Bukit Tinggi, Bentong).
- **Not placed**, because GeoNames has no such settlement: Lojing (KTN02),
  Belum (PRK04), Temanggong (SBH01), Terusan (SBH02 — GeoNames' only village of
  that name is in Kudat), Balong (SBH04), Meludam (SWK07), and Kampung
  Patarikan, **SWK09**'s only area. Every zone but SWK09 has other points;
  SWK09 can be chosen from the list but is never detected.
- Nearest-point mapping can be wrong near a boundary — Cyberjaya, in Sepang
  (SGR01), is nearer Putrajaya (WLY01) — which is why the detected zone is
  always shown for confirmation and can always be changed.
