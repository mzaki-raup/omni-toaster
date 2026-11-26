//! The fallback chain and the source label (FEAT-0055, `docs/11-prayer-and-azan.md` §7).
//!
//! One function decides which times a day shows and where they came from, so
//! the scheduler sees a list of times plus a label and nothing else:
//!
//! 1. a fresh JAKIM cache entry for today → use it;
//! 2. a live fetch succeeds → store it and use it;
//! 3. a stale cache entry for today → use it, labelled with when it was fetched;
//! 4. the bundled dataset has the date → use it ("Offline data");
//! 5. calculated times → use them ("Calculated (fallback)");
//! 6. nothing → [`Resolution::Nothing`], and the caller keeps the last times.
//!
//! Step 3 is not in the spec's list. A cache row that is over 30 days old still
//! holds the times JAKIM published for today, and they are better than a
//! calculation that is a few minutes off, so it sits before the later links.
//!
//! The cache, the fetch and the bundled data are traits: this crate cannot
//! depend on `ot-store`, and the chain is tested without a network by stubs.

use jiff::civil::Date;
use jiff::{Timestamp, ToSpan};

use crate::calculated::{self, Asr, Place, Preset};
use crate::fetch::{self, Cancel, FetchError, Fetched, Range};
use crate::jakim::{Prayer, PrayerDay};

/// The user's source setting (spec §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceChoice {
    /// JAKIM inside Malaysia, calculated elsewhere.
    #[default]
    Auto,
    Jakim,
    Calculated,
}

/// Whether the cache can serve a zone today. The rule itself lives in
/// `ot-store`, which the binary maps to this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    NoEntryForToday,
    TooOld,
}

/// What the chain reads and writes of the cache.
pub trait Cache {
    fn freshness(&self, zone: &str, today: Date, now: Timestamp) -> Freshness;
    /// Today's row and when it was written.
    fn day(&self, zone: &str, date: Date) -> Option<(PrayerDay, Timestamp)>;
    fn store(&self, zone: &str, days: &[PrayerDay], fetched_at: Timestamp);
}

/// One live attempt. [`fetch::Client`] is the real one.
pub trait Fetch {
    fn fetch(&self, zone: &str, range: Range, cancel: &Cancel) -> Result<Fetched, FetchError>;
}

impl Fetch for fetch::Client {
    fn fetch(&self, zone: &str, range: Range, cancel: &Cancel) -> Result<Fetched, FetchError> {
        Self::fetch(self, zone, range, cancel)
    }
}

/// The dataset shipped in the exe, if the Phase 9 terms check allows one.
pub trait Bundled {
    fn day(&self, zone: &str, date: Date) -> Option<PrayerDay>;
}

/// What the calculated step needs: where, and by which method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calculation {
    pub place: Place,
    pub preset: Preset,
    pub asr: Asr,
}

/// Where a day's times came from: the source label, always shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// JAKIM e-Solat: "Source: JAKIM e-Solat", with when it was fetched.
    Jakim { fetched_at: Timestamp },
    /// JAKIM's data from the cache with no fetch behind it today: shown as
    /// JAKIM, with the (older) time it was fetched.
    JakimCached { fetched_at: Timestamp },
    /// The dataset in the exe: "Offline data".
    Bundled,
    /// Calculated. `fallback` is set when JAKIM was wanted and could not be
    /// had; `approximate` when it stands in for JAKIM in Malaysia.
    Calculated { fallback: bool, approximate: bool },
}

/// The Fluent keys the source labels use, in both locale files. A test holds
/// both files to this list.
pub const LABEL_KEYS: [&str; 7] = [
    "prayer-source-jakim",
    "prayer-source-jakim-cached",
    "prayer-source-bundled",
    "prayer-source-calculated",
    "prayer-source-calculated-approximate",
    "prayer-source-calculated-fallback",
    "prayer-source-calculated-fallback-approximate",
];

impl Source {
    /// The Fluent key of the label. A source with a fetch time has it as `$time`
    /// ([`Source::fetched_at`]); the UI formats it with the shared formatter.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Jakim { .. } => "prayer-source-jakim",
            Self::JakimCached { .. } => "prayer-source-jakim-cached",
            Self::Bundled => "prayer-source-bundled",
            Self::Calculated {
                fallback: false,
                approximate: false,
            } => "prayer-source-calculated",
            Self::Calculated {
                fallback: false,
                approximate: true,
            } => "prayer-source-calculated-approximate",
            Self::Calculated {
                fallback: true,
                approximate: false,
            } => "prayer-source-calculated-fallback",
            Self::Calculated {
                fallback: true,
                approximate: true,
            } => "prayer-source-calculated-fallback-approximate",
        }
    }

    /// When JAKIM's times were fetched, for the sources that have it.
    pub fn fetched_at(self) -> Option<Timestamp> {
        match self {
            Self::Jakim { fetched_at } | Self::JakimCached { fetched_at } => Some(fetched_at),
            _ => None,
        }
    }
}

/// One day's times, in the order of the day, and where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub date: Date,
    pub times: Vec<(Prayer, Timestamp)>,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Times(Resolved),
    /// Every link failed: the caller keeps the last known times and, once a
    /// day ([`should_toast`]), says so.
    Nothing,
}

/// What a day is asked for.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub zone: &'a str,
    pub choice: SourceChoice,
    /// Whether the location is in Malaysia: decides Auto, and "approximate".
    pub in_malaysia: bool,
    pub today: Date,
    pub now: Timestamp,
    /// `None` when no place is known, which leaves the calculated step out.
    pub calculation: Option<Calculation>,
}

/// The chain's three sources of JAKIM data.
pub struct Chain<'a> {
    pub cache: &'a dyn Cache,
    pub fetch: &'a dyn Fetch,
    pub bundled: Option<&'a dyn Bundled>,
}

/// All eight times of a JAKIM day, or `None` if one is not a real instant.
fn jakim_times(day: &PrayerDay) -> Option<Vec<(Prayer, Timestamp)>> {
    Prayer::ALL
        .iter()
        .map(|prayer| day.at(*prayer).map(|at| (*prayer, at)))
        .collect()
}

fn jakim(day: &PrayerDay, source: Source) -> Option<Resolved> {
    Some(Resolved {
        date: day.date,
        times: jakim_times(day)?,
        source,
    })
}

impl Chain<'_> {
    /// The times for `request.today`, by the order in the module docs. Blocks
    /// for a live fetch, so it runs on a worker thread; each step is logged.
    pub fn resolve(&self, request: &Request<'_>, cancel: &Cancel) -> Resolution {
        let wanted = match request.choice {
            SourceChoice::Auto => request.in_malaysia,
            SourceChoice::Jakim => true,
            SourceChoice::Calculated => false,
        };
        if wanted && let Some(found) = self.jakim_links(request, cancel) {
            return Resolution::Times(found);
        }
        match request.calculation {
            Some(calculation) => {
                let day = calculated::day(
                    request.today,
                    calculation.place,
                    calculation.preset,
                    calculation.asr,
                );
                let times: Vec<_> = [
                    (Prayer::Fajr, day.fajr),
                    (Prayer::Syuruk, day.syuruk),
                    (Prayer::Dhuhr, day.dhuhr),
                    (Prayer::Asr, day.asr),
                    (Prayer::Maghrib, day.maghrib),
                    (Prayer::Isha, day.isha),
                ]
                .into_iter()
                .filter_map(|(prayer, at)| Some((prayer, at?)))
                .collect();
                // A day with a time missing is a gap, not a day to show.
                if times.len() == 6 {
                    let approximate =
                        calculated::is_approximate(calculation.preset, request.in_malaysia);
                    tracing::info!(fallback = wanted, approximate, "prayer times: calculated");
                    return Resolution::Times(Resolved {
                        date: request.today,
                        times,
                        source: Source::Calculated {
                            fallback: wanted,
                            approximate,
                        },
                    });
                }
                tracing::warn!("prayer times: the sun does not reach every angle today");
            }
            None => tracing::warn!("prayer times: no place is known to calculate for"),
        }
        tracing::warn!("prayer times: nothing available");
        Resolution::Nothing
    }

    /// Steps 1 to 4: the JAKIM links.
    fn jakim_links(&self, request: &Request<'_>, cancel: &Cancel) -> Option<Resolved> {
        let zone = request.zone;
        if !fetch::valid_zone(zone) {
            tracing::warn!(zone, "prayer times: not a JAKIM zone code; skipping JAKIM");
            return None;
        }

        let cached = self.cache.day(zone, request.today);
        if self.cache.freshness(zone, request.today, request.now) == Freshness::Fresh
            && let Some((day, fetched_at)) = &cached
            && let Some(found) = jakim(
                day,
                Source::Jakim {
                    fetched_at: *fetched_at,
                },
            )
        {
            tracing::info!(zone, "prayer times: fresh cache");
            return Some(found);
        }

        match self.fetch.fetch(zone, fetch::range(request.today), cancel) {
            Ok(fetched) => {
                self.cache.store(zone, &fetched.days, request.now);
                let today = fetched.days.iter().find(|day| day.date == request.today);
                if let Some(found) = today.and_then(|day| {
                    jakim(
                        day,
                        Source::Jakim {
                            fetched_at: request.now,
                        },
                    )
                }) {
                    tracing::info!(zone, days = fetched.days.len(), "prayer times: live fetch");
                    return Some(found);
                }
                tracing::warn!(zone, "prayer times: the fetch had no usable day for today");
            }
            Err(error) => tracing::warn!(zone, %error, "prayer times: live fetch failed"),
        }

        if let Some((day, fetched_at)) = &cached
            && let Some(found) = jakim(
                day,
                Source::JakimCached {
                    fetched_at: *fetched_at,
                },
            )
        {
            tracing::warn!(zone, %fetched_at, "prayer times: using the cache entry past its age");
            return Some(found);
        }

        let day = self.bundled?.day(zone, request.today)?;
        let found = jakim(&day, Source::Bundled)?;
        tracing::warn!(zone, "prayer times: using the bundled dataset");
        Some(found)
    }
}

/// Whether to show the "no prayer times" error toast: once a day, however many
/// times the chain finds nothing.
pub fn should_toast(last_toast: Option<Date>, today: Date) -> bool {
    last_toast != Some(today)
}

/// The bundled dataset warns 30 days before its last date (spec §5).
pub fn bundled_expires_soon(last_bundled: Date, today: Date) -> bool {
    today
        .checked_add(30.days())
        .is_ok_and(|warn_from| warn_from >= last_bundled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jakim::{self, Response};
    use std::cell::RefCell;
    use std::collections::HashMap;

    const SAMPLE: &str = include_str!("../tests/fixtures/esolat-2026-11-20/SGR01-month.json");
    const EN_FTL: &str = include_str!("../../../locales/en-US/omni_toaster.ftl");
    const MS_FTL: &str = include_str!("../../../locales/ms-MY/omni_toaster.ftl");

    fn date(y: i16, m: i8, d: i8) -> Date {
        Date::new(y, m, d).expect("a date")
    }

    fn at(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn november() -> Vec<PrayerDay> {
        match jakim::parse(SAMPLE.as_bytes()).expect("parses") {
            Response::Days { days, .. } => days,
            Response::NoRecord => panic!("no record"),
        }
    }

    /// A cache in a map, recording what it was asked to store.
    #[derive(Default)]
    struct MemoryCache {
        rows: RefCell<HashMap<(String, Date), (PrayerDay, Timestamp)>>,
        stores: RefCell<u32>,
    }

    impl MemoryCache {
        fn with(days: &[PrayerDay], fetched_at: Timestamp) -> Self {
            let cache = Self::default();
            cache.store("SGR01", days, fetched_at);
            *cache.stores.borrow_mut() = 0;
            cache
        }
    }

    impl Cache for MemoryCache {
        fn freshness(&self, zone: &str, today: Date, now: Timestamp) -> Freshness {
            match self.day(zone, today) {
                None => Freshness::NoEntryForToday,
                Some((_, fetched)) if now.duration_since(fetched).as_hours() < 30 * 24 => {
                    Freshness::Fresh
                }
                Some(_) => Freshness::TooOld,
            }
        }
        fn day(&self, zone: &str, date: Date) -> Option<(PrayerDay, Timestamp)> {
            self.rows.borrow().get(&(zone.to_string(), date)).cloned()
        }
        fn store(&self, zone: &str, days: &[PrayerDay], fetched_at: Timestamp) {
            *self.stores.borrow_mut() += 1;
            for day in days {
                self.rows
                    .borrow_mut()
                    .insert((zone.to_string(), day.date), (day.clone(), fetched_at));
            }
        }
    }

    /// A fetch that answers what it was told to, and counts its calls.
    struct StubFetch {
        answer: Result<Vec<PrayerDay>, FetchError>,
        calls: RefCell<u32>,
    }

    impl StubFetch {
        fn ok(days: Vec<PrayerDay>) -> Self {
            Self {
                answer: Ok(days),
                calls: RefCell::new(0),
            }
        }
        fn failing(error: FetchError) -> Self {
            Self {
                answer: Err(error),
                calls: RefCell::new(0),
            }
        }
    }

    impl Fetch for StubFetch {
        fn fetch(&self, zone: &str, range: Range, _: &Cancel) -> Result<Fetched, FetchError> {
            *self.calls.borrow_mut() += 1;
            self.answer.clone().map(|days| Fetched {
                zone: zone.to_string(),
                days,
                rejected: Vec::new(),
                requested: range,
            })
        }
    }

    struct StubBundled(Vec<PrayerDay>);

    impl Bundled for StubBundled {
        fn day(&self, _: &str, date: Date) -> Option<PrayerDay> {
            self.0.iter().find(|day| day.date == date).cloned()
        }
    }

    fn calculation() -> Calculation {
        Calculation {
            place: Place {
                latitude: 3.0851,
                longitude: 101.5328,
            },
            preset: Preset::MalaysiaJakim,
            asr: Asr::Shafii,
        }
    }

    fn request(
        choice: SourceChoice,
        in_malaysia: bool,
        with_calculation: bool,
    ) -> Request<'static> {
        Request {
            zone: "SGR01",
            choice,
            in_malaysia,
            today: date(2026, 11, 20),
            now: at("2026-11-20T01:00:00Z"),
            calculation: with_calculation.then(calculation),
        }
    }

    fn source(resolution: Resolution) -> Source {
        match resolution {
            Resolution::Times(resolved) => resolved.source,
            Resolution::Nothing => panic!("expected times, got nothing"),
        }
    }

    const FALLBACK: Source = Source::Calculated {
        fallback: true,
        approximate: true,
    };

    #[test]
    fn a_fresh_cache_is_used_without_a_fetch() {
        let fetched_at = at("2026-11-15T00:00:00Z");
        let cache = MemoryCache::with(&november(), fetched_at);
        let fetch = StubFetch::failing(FetchError::Network("must not be called".into()));
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        let resolved = chain.resolve(&request(SourceChoice::Auto, true, true), &Cancel::new());
        assert_eq!(source(resolved.clone()), Source::Jakim { fetched_at });
        assert_eq!(*fetch.calls.borrow(), 0);
        let Resolution::Times(times) = resolved else {
            panic!("times")
        };
        assert_eq!(times.times.len(), 8, "all eight JAKIM times");
        assert!(
            times.times.windows(2).all(|pair| pair[0].1 < pair[1].1),
            "in order"
        );
    }

    #[test]
    fn an_empty_cache_takes_a_live_fetch_and_stores_it() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::ok(november());
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        let req = request(SourceChoice::Auto, true, true);
        let found = source(chain.resolve(&req, &Cancel::new()));
        assert_eq!(
            found,
            Source::Jakim {
                fetched_at: req.now
            }
        );
        assert_eq!(*cache.stores.borrow(), 1, "the range was cached");
        assert!(cache.day("SGR01", date(2026, 11, 30)).is_some());
    }

    #[test]
    fn with_the_network_off_a_stale_cache_row_is_used_and_says_when() {
        let old = at("2026-09-01T00:00:00Z");
        let cache = MemoryCache::with(&november(), old);
        let fetch = StubFetch::failing(FetchError::Network("offline".into()));
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        assert_eq!(
            source(chain.resolve(&request(SourceChoice::Auto, true, true), &Cancel::new())),
            Source::JakimCached { fetched_at: old }
        );
    }

    #[test]
    fn every_failure_with_no_cache_falls_to_calculated_and_says_so() {
        for error in [
            FetchError::Network("offline".into()),
            FetchError::Http(500),
            FetchError::Parse(jakim::ParseError::NotJson("<html>".into())),
            FetchError::TooLarge,
            FetchError::NoRecord,
        ] {
            let cache = MemoryCache::default();
            let fetch = StubFetch::failing(error.clone());
            let chain = Chain {
                cache: &cache,
                fetch: &fetch,
                bundled: None,
            };
            assert_eq!(
                source(chain.resolve(&request(SourceChoice::Auto, true, true), &Cancel::new())),
                FALLBACK,
                "{error}"
            );
        }
    }

    #[test]
    fn the_bundled_dataset_is_used_when_it_has_the_date_and_skipped_when_it_does_not() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::failing(FetchError::NoRecord);
        let with = StubBundled(november());
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: Some(&with),
        };
        let req = request(SourceChoice::Auto, true, true);
        assert_eq!(source(chain.resolve(&req, &Cancel::new())), Source::Bundled);

        // Present but without today's date.
        let without_date = StubBundled(vec![]);
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: Some(&without_date),
        };
        assert_eq!(source(chain.resolve(&req, &Cancel::new())), FALLBACK);

        // Absent altogether.
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        assert_eq!(source(chain.resolve(&req, &Cancel::new())), FALLBACK);
    }

    #[test]
    fn a_fetch_that_answers_without_today_is_a_miss_not_a_success() {
        let cache = MemoryCache::default();
        let tomorrow_on: Vec<_> = november()
            .into_iter()
            .filter(|d| d.date > date(2026, 11, 20))
            .collect();
        let fetch = StubFetch::ok(tomorrow_on);
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        assert_eq!(
            source(chain.resolve(&request(SourceChoice::Auto, true, true), &Cancel::new())),
            FALLBACK
        );
    }

    #[test]
    fn calculated_by_choice_or_abroad_is_not_a_fallback() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::failing(FetchError::Network("must not be called".into()));
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };

        // Chosen, in Malaysia: approximate, not a fallback.
        assert_eq!(
            source(chain.resolve(
                &request(SourceChoice::Calculated, true, true),
                &Cancel::new()
            )),
            Source::Calculated {
                fallback: false,
                approximate: true
            }
        );
        // Auto abroad: calculated, and JAKIM is never asked.
        let mut abroad = request(SourceChoice::Auto, false, true);
        abroad.calculation = Some(Calculation {
            preset: Preset::MuslimWorldLeague,
            ..calculation()
        });
        assert_eq!(
            source(chain.resolve(&abroad, &Cancel::new())),
            Source::Calculated {
                fallback: false,
                approximate: false
            }
        );
        assert_eq!(*fetch.calls.borrow(), 0);
    }

    #[test]
    fn jakim_by_choice_abroad_asks_jakim() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::ok(november());
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        let req = request(SourceChoice::Jakim, false, true);
        assert!(matches!(
            source(chain.resolve(&req, &Cancel::new())),
            Source::Jakim { .. }
        ));
    }

    #[test]
    fn nothing_when_every_link_fails_and_no_place_is_known() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::failing(FetchError::Http(500));
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        assert_eq!(
            chain.resolve(&request(SourceChoice::Auto, true, false), &Cancel::new()),
            Resolution::Nothing
        );
    }

    #[test]
    fn a_day_the_sun_does_not_cover_is_nothing_not_a_partial_day() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::failing(FetchError::Http(500));
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        let mut polar = request(SourceChoice::Calculated, false, true);
        polar.today = date(2026, 6, 21);
        polar.calculation = Some(Calculation {
            place: Place {
                latitude: 69.6492,
                longitude: 18.9553,
            },
            preset: Preset::MuslimWorldLeague,
            asr: Asr::Shafii,
        });
        assert_eq!(chain.resolve(&polar, &Cancel::new()), Resolution::Nothing);
    }

    #[test]
    fn a_bad_zone_code_skips_jakim_without_a_request() {
        let cache = MemoryCache::default();
        let fetch = StubFetch::ok(november());
        let chain = Chain {
            cache: &cache,
            fetch: &fetch,
            bundled: None,
        };
        let mut req = request(SourceChoice::Auto, true, true);
        req.zone = "";
        assert_eq!(source(chain.resolve(&req, &Cancel::new())), FALLBACK);
        assert_eq!(*fetch.calls.borrow(), 0);
    }

    #[test]
    fn the_error_toast_comes_once_a_day() {
        let today = date(2026, 11, 20);
        assert!(should_toast(None, today));
        assert!(!should_toast(Some(today), today));
        assert!(should_toast(Some(date(2026, 11, 19)), today));
    }

    #[test]
    fn the_bundled_warning_starts_thirty_days_before_the_last_date() {
        let last = date(2027, 12, 31);
        assert!(!bundled_expires_soon(last, date(2027, 11, 30)));
        assert!(bundled_expires_soon(last, date(2027, 12, 1)));
        assert!(bundled_expires_soon(last, date(2027, 12, 31)));
    }

    #[test]
    fn every_label_has_a_key_in_both_languages_and_the_keys_are_distinct() {
        let sources = [
            Source::Jakim {
                fetched_at: at("2026-11-20T00:00:00Z"),
            },
            Source::JakimCached {
                fetched_at: at("2026-11-20T00:00:00Z"),
            },
            Source::Bundled,
            Source::Calculated {
                fallback: false,
                approximate: false,
            },
            Source::Calculated {
                fallback: false,
                approximate: true,
            },
            Source::Calculated {
                fallback: true,
                approximate: false,
            },
            Source::Calculated {
                fallback: true,
                approximate: true,
            },
        ];
        let keys: Vec<_> = sources.iter().map(|s| s.label_key()).collect();
        assert_eq!(
            keys, LABEL_KEYS,
            "every label's key is in LABEL_KEYS, in order"
        );
        for key in LABEL_KEYS {
            for (name, file) in [("en-US", EN_FTL), ("ms-MY", MS_FTL)] {
                assert!(
                    file.lines()
                        .any(|line| line.starts_with(&format!("{key} ="))),
                    "{key} is missing from {name}"
                );
            }
        }
    }
}
