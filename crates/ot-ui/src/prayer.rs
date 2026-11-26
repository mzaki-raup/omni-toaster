//! The data side of Settings ▸ Prayer (FEAT-0057, `docs/11-prayer-and-azan.md`).
//!
//! The pane draws; this decides and fetches. [`plan`] turns the settings and the
//! saved place into what to ask for. [`Controller`] runs each piece of blocking
//! work — a store read, an IP lookup, a city search, a day's resolve — on a
//! short-lived thread of its own and sends the answer back over a **bounded**
//! channel the window drains at frame start. Each kind of work has its own
//! thread, so a ten-second e-Solat fetch never holds a city search behind it
//! (`.claude/rules/threads-and-scheduling.md`).
//!
//! A reply to a search or a resolve that a newer one has replaced is dropped,
//! and a new resolve cancels the one in flight, so changing the zone twice
//! quickly leaves the second zone's times on screen, not whichever finished last.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use ot_core::PrayerSettings;
use ot_prayer::calculated::{Asr, Place, Preset};
use ot_prayer::chain::{
    Cache, Calculation, Chain, Fetch, Freshness, Request, Resolution, SourceChoice,
};
use ot_prayer::fetch::{self, Cancel};
use ot_prayer::jakim::PrayerDay;
use ot_prayer::location::{Consent, Located, Lookup, LookupError, MALAYSIA_TZ};
use ot_prayer::zones::{self, Zone};
use ot_store::Store;
use ot_store::location::{Origin, StoredLocation};
use ot_store::prayer::{CachedDay, Freshness as StoreFreshness, Origin as CacheOrigin};

/// Something the pane should say to the user before the times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    /// JAKIM is chosen and there is no zone: nothing to ask for.
    JakimNeedsAZone,
    /// JAKIM is chosen for a place outside Malaysia: times follow the zone
    /// chosen, which may differ from local custom.
    JakimAbroad,
}

/// What to ask for, from the settings and the saved place.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub source: SourceChoice,
    /// The zone JAKIM is asked about: the one the user chose, else the one
    /// detected from the place.
    pub zone: Option<&'static Zone>,
    /// Whether [`Plan::zone`] came from the place and not from the user.
    pub zone_detected: bool,
    pub in_malaysia: bool,
    /// `None` while there is no place and no zone to calculate for.
    pub calculation: Option<Calculation>,
    /// The zone a calculated day is read in: the place's own, or Malaysia's.
    pub tz: String,
    pub warning: Option<Warning>,
}

/// The first point of a zone, which stands in for a place when only the zone is
/// known — a user who chose a zone and never gave a location.
fn zone_point(zone: &Zone) -> Option<Place> {
    zones::points()
        .iter()
        .find(|point| point.code == zone.code)
        .map(|point| Place {
            latitude: point.latitude,
            longitude: point.longitude,
        })
}

/// Decide what to ask for. A zone the user chose overrides the detected one. Whether
/// this is Malaysia follows the saved place; a chosen zone counts only when there
/// is no place.
pub fn plan(settings: &PrayerSettings, saved: Option<&StoredLocation>) -> Plan {
    let source = SourceChoice::parse(&settings.source);
    let chosen = settings.zone.as_deref().and_then(zones::zone);
    let detected = saved.and_then(|place| {
        zones::detect(
            place.latitude,
            place.longitude,
            place.country_code.as_deref(),
        )
    });
    let (zone, zone_detected) = match (chosen, detected) {
        (Some(zone), _) => (Some(zone), false),
        (None, Some(zone)) => (Some(zone), true),
        (None, None) => (None, false),
    };
    let place_in_malaysia = saved.is_some_and(|place| {
        zones::in_malaysia(
            place.latitude,
            place.longitude,
            place.country_code.as_deref(),
        )
    });
    // Where the user is decides, not an old choice: a zone picked in Malaysia
    // stays in the settings after a move abroad. With no place at all, a chosen
    // zone is all there is to go on.
    let in_malaysia = match saved {
        Some(_) => place_in_malaysia,
        None => chosen.is_some(),
    };

    let place = saved
        .map(|place| Place {
            latitude: place.latitude,
            longitude: place.longitude,
        })
        .or_else(|| zone.and_then(zone_point));
    let preset = settings
        .preset
        .as_deref()
        .and_then(Preset::parse)
        .unwrap_or_else(|| Preset::default_for(in_malaysia));
    let calculation = place.map(|place| Calculation {
        place,
        preset,
        asr: Asr::parse(&settings.asr),
    });

    let warning = match (source, zone) {
        (SourceChoice::Jakim, None) => Some(Warning::JakimNeedsAZone),
        (SourceChoice::Jakim, Some(_)) if saved.is_some() && !place_in_malaysia => {
            Some(Warning::JakimAbroad)
        }
        _ => None,
    };
    // A place outside Malaysia is read in its own zone; Malaysia is UTC+8
    // whatever the place says, and so is JAKIM's data.
    let tz = match saved {
        Some(place) if !place_in_malaysia => place.tz.clone(),
        _ => MALAYSIA_TZ.to_string(),
    };
    Plan {
        source,
        zone,
        zone_detected,
        in_malaysia,
        calculation,
        tz,
        warning,
    }
}

/// Today in the plan's zone: the date a day's times are for. The PC's own zone
/// is never consulted.
pub fn today(plan: &Plan, now: Timestamp) -> Date {
    TimeZone::get(&plan.tz)
        .or_else(|_| TimeZone::get(MALAYSIA_TZ))
        .map_or_else(
            |_| now.to_zoned_datetime(TimeZone::UTC).date(),
            |tz| now.to_zoned_datetime(tz).date(),
        )
}

/// The chain's cache, over the store. A newtype, because the trait and the
/// store are both foreign to this crate. A store error is logged and read as a
/// miss: the chain then goes on to the next link.
pub struct StoreCache<'a>(pub &'a Store);

impl Cache for StoreCache<'_> {
    fn freshness(&self, zone: &str, today: Date, now: Timestamp) -> Freshness {
        match self.0.prayer_freshness(zone, today, now) {
            Ok(StoreFreshness::Fresh) => Freshness::Fresh,
            Ok(StoreFreshness::NoEntryForToday) => Freshness::NoEntryForToday,
            Ok(StoreFreshness::TooOld) => Freshness::TooOld,
            Err(error) => {
                tracing::warn!(%error, "the prayer cache could not be read");
                Freshness::NoEntryForToday
            }
        }
    }

    fn day(&self, zone: &str, date: Date) -> Option<(PrayerDay, Timestamp)> {
        match self.0.prayer_day(zone, date) {
            Ok(found) => found.map(|cached| {
                (
                    PrayerDay {
                        date: cached.day.date,
                        hijri: cached.day.hijri,
                        times: cached.day.times,
                    },
                    cached.fetched_at,
                )
            }),
            Err(error) => {
                tracing::warn!(%error, "the prayer cache could not be read");
                None
            }
        }
    }

    fn store(&self, zone: &str, days: &[PrayerDay], fetched_at: Timestamp) {
        let cached: Vec<CachedDay> = days
            .iter()
            .map(|day| CachedDay {
                date: day.date,
                hijri: day.hijri.clone(),
                times: day.times,
            })
            .collect();
        if let Err(error) = self
            .0
            .write_prayer_days(zone, &cached, CacheOrigin::Api, fetched_at)
        {
            tracing::warn!(%error, "the fetched prayer times could not be cached");
        }
    }
}

/// One day for a plan: the chain, over this store and this fetch. Blocking.
pub fn resolve_day(
    store: &Store,
    fetch: &dyn Fetch,
    plan: &Plan,
    now: Timestamp,
    cancel: &Cancel,
) -> Resolution {
    let cache = StoreCache(store);
    let chain = Chain {
        cache: &cache,
        fetch,
        bundled: None,
    };
    let request = Request {
        zone: plan.zone.map_or("", |zone| zone.code.as_str()),
        choice: plan.source,
        in_malaysia: plan.in_malaysia,
        today: today(plan, now),
        now,
        calculation: plan.calculation,
    };
    chain.resolve(&request, cancel)
}

/// A place in the form the store keeps.
pub fn stored(place: &Located, origin: Origin, now: Timestamp) -> StoredLocation {
    StoredLocation {
        name: place.name.clone(),
        latitude: place.latitude,
        longitude: place.longitude,
        tz: place.tz.clone(),
        country_code: place.country_code.clone(),
        origin,
        updated_at: now,
    }
}

/// What a worker sends back.
#[derive(Debug)]
pub enum Reply {
    /// The saved place, after a load or a save.
    Location(Option<StoredLocation>),
    /// The IP lookup found nothing; the reason, for the status line.
    DetectFailed(LookupError),
    Search(Result<Vec<Located>, LookupError>),
    Day(Resolution),
    /// A prayer's template, read or just saved.
    Template {
        prayer: String,
        template: Option<ot_store::reminders::Template>,
    },
    /// A store or thread failure the pane should say out loud.
    Failed(String),
}

/// What the replies are tagged with, so a stale one can be recognised.
#[derive(Debug)]
struct Tagged {
    search: Option<u64>,
    day: Option<u64>,
    reply: Reply,
}

/// The window's side: starts the work, drains the answers.
pub struct Controller {
    root: PathBuf,
    sender: SyncSender<Tagged>,
    receiver: Receiver<Tagged>,
    search_generation: u64,
    day_generation: u64,
    in_flight: Option<Cancel>,
}

impl Controller {
    pub fn new(root: &Path) -> Self {
        let (sender, receiver) = sync_channel(16);
        Self {
            root: root.to_path_buf(),
            sender,
            receiver,
            search_generation: 0,
            day_generation: 0,
            in_flight: None,
        }
    }

    fn spawn(&self, name: &str, work: impl FnOnce(&Path, &SyncSender<Tagged>) + Send + 'static) {
        let root = self.root.clone();
        let sender = self.sender.clone();
        if let Err(error) = std::thread::Builder::new()
            .name(name.to_string())
            .spawn(move || work(&root, &sender))
        {
            tracing::error!(%error, thread = name, "a prayer thread could not be started");
        }
    }

    /// Read the saved place.
    pub fn load_location(&self) {
        self.spawn("prayer-location", |root, out| {
            let reply = match Store::open(root).and_then(|store| store.location()) {
                Ok(place) => Reply::Location(place),
                Err(error) => Reply::Failed(error.to_string()),
            };
            send(out, None, None, reply);
        });
    }

    /// Look the place up from the IP address. **Makes no request unless
    /// `consent` is granted**; the answer is saved as a detection, which never
    /// replaces a place the user chose.
    pub fn detect(&self, consent: Consent) {
        self.spawn("prayer-detect", move |root, out| {
            let now = Timestamp::now();
            let reply = match Lookup::new().detect(consent) {
                Err(error) => Reply::DetectFailed(error),
                Ok(found) => match Store::open(root).and_then(|store| {
                    store.save_detected_location(&stored(&found, Origin::Ip, now))?;
                    store.location()
                }) {
                    Ok(place) => Reply::Location(place),
                    Err(error) => Reply::Failed(error.to_string()),
                },
            };
            send(out, None, None, reply);
        });
    }

    /// Save a place the user chose, which replaces any detected one.
    pub fn save_manual(&self, place: Located) {
        self.spawn("prayer-save-location", move |root, out| {
            let now = Timestamp::now();
            let reply = match Store::open(root).and_then(|store| {
                store.save_manual_location(&stored(&place, Origin::Manual, now))?;
                store.location()
            }) {
                Ok(saved) => Reply::Location(saved),
                Err(error) => Reply::Failed(error.to_string()),
            };
            send(out, None, None, reply);
        });
    }

    /// Search for a city. Replaces any search still running: its answer, when it
    /// comes, is dropped.
    pub fn search(&mut self, text: String, language: &'static str) {
        self.search_generation += 1;
        let generation = self.search_generation;
        self.spawn("prayer-search", move |_, out| {
            let reply = Reply::Search(Lookup::new().search(&text, language));
            send(out, Some(generation), None, reply);
        });
    }

    /// Work out today's times for `plan`. Cancels the resolve in flight.
    pub fn resolve(&mut self, plan: Plan) {
        self.cancel();
        self.day_generation += 1;
        let generation = self.day_generation;
        let cancel = Cancel::new();
        self.in_flight = Some(cancel.clone());
        self.spawn("prayer-resolve", move |root, out| {
            let now = Timestamp::now();
            let reply = match Store::open(root) {
                Ok(store) => Reply::Day(resolve_day(
                    &store,
                    &fetch::Client::new(),
                    &plan,
                    now,
                    &cancel,
                )),
                Err(error) => Reply::Failed(error.to_string()),
            };
            // A cancelled resolve delivers nothing: its window has moved on.
            if !cancel.is_cancelled() {
                send(out, None, Some(generation), reply);
            }
        });
    }

    /// Read a prayer's template.
    pub fn load_template(&self, prayer: String) {
        self.spawn("prayer-template", move |root, out| {
            let reply = match Store::open(root).and_then(|store| store.template_for_prayer(&prayer))
            {
                Ok(template) => Reply::Template { prayer, template },
                Err(error) => Reply::Failed(error.to_string()),
            };
            send(out, None, None, reply);
        });
    }

    /// Save what the user wrote for a prayer's template, and answer with it as
    /// it now stands.
    pub fn save_template(&self, prayer: String, edit: ot_store::reminders::PrayerTemplateEdit) {
        self.spawn("prayer-template-save", move |root, out| {
            let saved = Store::open(root).and_then(|store| {
                store.set_prayer_template(&prayer, &edit)?;
                store.template_for_prayer(&prayer)
            });
            let reply = match saved {
                Ok(template) => Reply::Template { prayer, template },
                Err(error) => Reply::Failed(error.to_string()),
            };
            send(out, None, None, reply);
        });
    }

    /// Stop the resolve in flight, for a window that is closing.
    pub fn cancel(&mut self) {
        if let Some(cancel) = self.in_flight.take() {
            cancel.cancel();
        }
    }

    /// Every answer that has arrived and is still wanted, oldest first.
    pub fn drain(&mut self) -> Vec<Reply> {
        let mut replies = Vec::new();
        while let Ok(tagged) = self.receiver.try_recv() {
            let stale = tagged.search.is_some_and(|g| g != self.search_generation)
                || tagged.day.is_some_and(|g| g != self.day_generation);
            if !stale {
                replies.push(tagged.reply);
            }
        }
        replies
    }
}

/// Send, and on a full channel say so: the window is behind, and what is lost is
/// a reply it will ask for again.
fn send(out: &SyncSender<Tagged>, search: Option<u64>, day: Option<u64>, reply: Reply) {
    if let Err(TrySendError::Full(_)) = out.try_send(Tagged { search, day, reply }) {
        tracing::warn!("a prayer reply was dropped: the window is not keeping up");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_prayer::chain::Source;
    use ot_prayer::fetch::{FetchError, Fetched, Range};
    use ot_prayer::jakim::{self, Response};
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    const SGR01: &str =
        include_str!("../../ot-prayer/tests/fixtures/esolat-2026-11-20/SGR01-month.json");

    fn at(text: &str) -> Timestamp {
        text.parse().expect("an instant")
    }

    fn november() -> Vec<PrayerDay> {
        match jakim::parse(SGR01.as_bytes()).expect("parses") {
            Response::Days { days, .. } => days,
            Response::NoRecord => panic!("no record"),
        }
    }

    fn saved(name: &str, lat: f64, lon: f64, tz: &str, country: &str) -> StoredLocation {
        StoredLocation {
            name: name.to_string(),
            latitude: lat,
            longitude: lon,
            tz: tz.to_string(),
            country_code: Some(country.to_string()),
            origin: Origin::Manual,
            updated_at: at("2026-11-26T01:00:00Z"),
        }
    }

    fn shah_alam() -> StoredLocation {
        saved("Shah Alam", 3.09, 101.53, MALAYSIA_TZ, "MY")
    }

    fn london() -> StoredLocation {
        saved("London", 51.51, -0.13, "Europe/London", "GB")
    }

    fn settings(source: &str, zone: Option<&str>) -> PrayerSettings {
        PrayerSettings {
            source: source.to_string(),
            zone: zone.map(str::to_string),
            ..PrayerSettings::default()
        }
    }

    struct StubFetch {
        answer: Result<Vec<PrayerDay>, FetchError>,
        calls: RefCell<u32>,
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

    #[test]
    fn the_zone_is_detected_from_the_place_and_a_chosen_one_overrides_it() {
        let detected = plan(&settings("auto", None), Some(&shah_alam()));
        assert_eq!(detected.zone.map(|z| z.code.as_str()), Some("SGR01"));
        assert!(detected.zone_detected && detected.in_malaysia);

        let chosen = plan(&settings("auto", Some("WLY01")), Some(&shah_alam()));
        assert_eq!(chosen.zone.map(|z| z.code.as_str()), Some("WLY01"));
        assert!(!chosen.zone_detected, "the user's own choice");

        // A zone alone, with no place, is enough to count as Malaysia and to
        // calculate for.
        let only_zone = plan(&settings("auto", Some("JHR01")), None);
        assert!(only_zone.in_malaysia);
        assert!(
            only_zone.calculation.is_some(),
            "a point in the zone stands in"
        );

        // A code nobody recognises is no choice at all.
        let bad = plan(&settings("auto", Some("XXX99")), Some(&shah_alam()));
        assert_eq!(bad.zone.map(|z| z.code.as_str()), Some("SGR01"));
    }

    #[test]
    fn auto_follows_the_place_so_an_old_chosen_zone_does_not_pull_jakim_abroad() {
        // A zone chosen in Malaysia stays in the settings after the user moves.
        // Auto follows where they are (spec section 1), so in London it
        // calculates for London, with the default method and no JAKIM request.
        let abroad = plan(&settings("auto", Some("SGR01")), Some(&london()));
        assert!(!abroad.in_malaysia, "the place decides, not the old zone");
        assert_eq!(
            abroad.calculation.expect("a place").preset,
            Preset::MuslimWorldLeague
        );
        assert_eq!(abroad.tz, "Europe/London");
        assert_eq!(
            abroad.zone.map(|z| z.code.as_str()),
            Some("SGR01"),
            "still the chosen zone, for when the user picks JAKIM"
        );

        // With no place at all, a chosen zone is all there is to go on.
        assert!(plan(&settings("auto", Some("SGR01")), None).in_malaysia);

        // And the chain asks JAKIM nothing for that plan.
        let store = Store::open_in_memory().expect("opens");
        let fetch = StubFetch {
            answer: Err(FetchError::Network("must not be called".into())),
            calls: RefCell::new(0),
        };
        let resolved = resolve_day(
            &store,
            &fetch,
            &abroad,
            at("2026-11-20T01:00:00Z"),
            &Cancel::new(),
        );
        let Resolution::Times(resolved) = resolved else {
            panic!("times")
        };
        assert_eq!(
            resolved.source,
            Source::Calculated {
                fallback: false,
                approximate: false
            }
        );
        assert_eq!(*fetch.calls.borrow(), 0);
    }

    #[test]
    fn abroad_there_is_no_zone_and_calculation_uses_the_places_own_time_zone() {
        let abroad = plan(&settings("auto", None), Some(&london()));
        assert_eq!(abroad.zone, None);
        assert!(!abroad.in_malaysia);
        assert_eq!(abroad.tz, "Europe/London");
        let calculation = abroad.calculation.expect("a place to calculate for");
        assert_eq!(
            calculation.preset,
            Preset::MuslimWorldLeague,
            "the default abroad"
        );

        let home = plan(&settings("auto", None), Some(&shah_alam()));
        assert_eq!(home.tz, MALAYSIA_TZ);
        assert_eq!(
            home.calculation.expect("a place").preset,
            Preset::MalaysiaJakim
        );

        let nothing = plan(&settings("auto", None), None);
        assert_eq!((nothing.zone, nothing.calculation), (None, None));
    }

    #[test]
    fn jakim_with_no_zone_or_abroad_is_warned_about() {
        assert_eq!(
            plan(&settings("jakim", None), Some(&london())).warning,
            Some(Warning::JakimNeedsAZone)
        );
        assert_eq!(
            plan(&settings("jakim", Some("SGR01")), Some(&london())).warning,
            Some(Warning::JakimAbroad)
        );
        assert_eq!(
            plan(&settings("jakim", None), Some(&shah_alam())).warning,
            None
        );
        assert_eq!(plan(&settings("auto", None), Some(&london())).warning, None);
    }

    #[test]
    fn a_stored_preset_and_asar_are_used_and_an_unknown_one_is_ignored() {
        let mut stored = settings("calculated", None);
        stored.preset = Some("umm-al-qura".to_string());
        stored.asr = "hanafi".to_string();
        let calculation = plan(&stored, Some(&london())).calculation.expect("a place");
        assert_eq!(
            (calculation.preset, calculation.asr),
            (Preset::UmmAlQura, Asr::Hanafi)
        );

        stored.preset = Some("egyptian".to_string());
        stored.asr = "maliki".to_string();
        let calculation = plan(&stored, Some(&london())).calculation.expect("a place");
        assert_eq!(
            (calculation.preset, calculation.asr),
            (Preset::MuslimWorldLeague, Asr::Shafii)
        );
    }

    #[test]
    fn today_is_the_date_in_the_places_zone_not_the_pcs() {
        // 17:00 UTC on the 20th is already the 21st in Kuala Lumpur (01:00) and
        // still the 20th in London.
        let now = at("2026-11-20T17:00:00Z");
        let kl = plan(&settings("auto", None), Some(&shah_alam()));
        assert_eq!(today(&kl, now).to_string(), "2026-11-21");
        let uk = plan(&settings("auto", None), Some(&london()));
        assert_eq!(today(&uk, now).to_string(), "2026-11-20");
    }

    #[test]
    fn the_store_cache_round_trips_a_day_and_judges_freshness() {
        let store = Store::open_in_memory().expect("opens");
        let cache = StoreCache(&store);
        let days = november();
        let fetched = at("2026-11-20T01:00:00Z");
        cache.store("SGR01", &days, fetched);

        let (day, when) = cache.day("SGR01", days[0].date).expect("the day is there");
        assert_eq!((day, when), (days[0].clone(), fetched));
        assert_eq!(
            cache.freshness("SGR01", days[0].date, fetched),
            Freshness::Fresh
        );
        assert_eq!(
            cache.freshness(
                "SGR01",
                days[0].date,
                fetched + jiff::SignedDuration::from_hours(31 * 24)
            ),
            Freshness::TooOld
        );
        assert_eq!(
            cache.freshness("WLY01", days[0].date, fetched),
            Freshness::NoEntryForToday
        );
    }

    #[test]
    fn a_resolve_fetches_caches_and_then_serves_the_cache_with_the_network_down() {
        let store = Store::open_in_memory().expect("opens");
        let plan = plan(&settings("auto", None), Some(&shah_alam()));
        let now = at("2026-11-20T01:00:00Z");

        let up = StubFetch {
            answer: Ok(november()),
            calls: RefCell::new(0),
        };
        let first = resolve_day(&store, &up, &plan, now, &Cancel::new());
        let Resolution::Times(first) = first else {
            panic!("times")
        };
        assert_eq!(first.source, Source::Jakim { fetched_at: now });
        assert_eq!(*up.calls.borrow(), 1);

        // Later the same day, the network is down: the cache answers and the
        // fetch is never asked.
        let down = StubFetch {
            answer: Err(FetchError::Network("offline".into())),
            calls: RefCell::new(0),
        };
        let later = resolve_day(
            &store,
            &down,
            &plan,
            now + jiff::SignedDuration::from_hours(3),
            &Cancel::new(),
        );
        let Resolution::Times(later) = later else {
            panic!("times")
        };
        assert_eq!(later.source, Source::Jakim { fetched_at: now });
        assert_eq!(later.times, first.times, "the same times, from the cache");
        assert_eq!(*down.calls.borrow(), 0, "a fresh cache needs no fetch");
    }

    #[test]
    fn with_no_cache_and_no_network_the_plan_falls_back_to_calculated() {
        let store = Store::open_in_memory().expect("opens");
        let plan = plan(&settings("auto", None), Some(&shah_alam()));
        let down = StubFetch {
            answer: Err(FetchError::Network("offline".into())),
            calls: RefCell::new(0),
        };
        let resolved = resolve_day(
            &store,
            &down,
            &plan,
            at("2026-11-20T01:00:00Z"),
            &Cancel::new(),
        );
        let Resolution::Times(resolved) = resolved else {
            panic!("times")
        };
        assert_eq!(
            resolved.source,
            Source::Calculated {
                fallback: true,
                approximate: true
            }
        );
    }

    fn wait_for(controller: &mut Controller, want: usize) -> Vec<Reply> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut replies = Vec::new();
        while replies.len() < want && Instant::now() < deadline {
            replies.extend(controller.drain());
            std::thread::sleep(Duration::from_millis(10));
        }
        replies
    }

    #[test]
    fn the_controller_saves_and_reads_a_place_and_resolves_off_the_calling_thread() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        std::fs::create_dir_all(dir.path().join("data")).expect("data folder");
        let mut controller = Controller::new(dir.path());

        controller.load_location();
        let replies = wait_for(&mut controller, 1);
        assert!(
            matches!(replies.as_slice(), [Reply::Location(None)]),
            "{replies:?}"
        );

        controller.save_manual(Located {
            name: "Shah Alam".to_string(),
            latitude: 3.09,
            longitude: 101.53,
            tz: MALAYSIA_TZ.to_string(),
            country_code: Some("MY".to_string()),
        });
        let replies = wait_for(&mut controller, 1);
        let [Reply::Location(Some(place))] = replies.as_slice() else {
            panic!("{replies:?}")
        };
        assert_eq!(
            (place.name.as_str(), place.origin),
            ("Shah Alam", Origin::Manual)
        );

        // Calculated by choice: no network is touched.
        controller.resolve(plan(&settings("calculated", None), Some(place)));
        let replies = wait_for(&mut controller, 1);
        assert!(
            matches!(replies.as_slice(), [Reply::Day(Resolution::Times(_))]),
            "{replies:?}"
        );
    }

    #[test]
    fn a_newer_resolve_replaces_an_older_one_and_only_its_answer_arrives() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        std::fs::create_dir_all(dir.path().join("data")).expect("data folder");
        // As in the app, where `main` opens the store before any window thread:
        // two threads creating a new database at once can fail on the WAL switch.
        drop(Store::open(dir.path()).expect("the store is created first"));
        let mut controller = Controller::new(dir.path());
        let place = shah_alam();
        // Both are calculated, so neither waits on a network; the first is
        // cancelled and the second is the generation that counts.
        controller.resolve(plan(&settings("calculated", None), Some(&place)));
        controller.resolve(plan(&settings("calculated", Some("WLY01")), Some(&place)));
        let replies = wait_for(&mut controller, 1);
        std::thread::sleep(Duration::from_millis(300));
        let late = controller.drain();
        assert_eq!(
            replies.len() + late.len(),
            1,
            "exactly one answer: {replies:?} {late:?}"
        );
    }

    #[test]
    fn detect_without_consent_sends_a_refusal_and_saves_nothing() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        std::fs::create_dir_all(dir.path().join("data")).expect("data folder");
        let mut controller = Controller::new(dir.path());
        controller.detect(Consent::Unset);
        let replies = wait_for(&mut controller, 1);
        assert!(
            matches!(
                replies.as_slice(),
                [Reply::DetectFailed(LookupError::NoConsent)]
            ),
            "{replies:?}"
        );
        assert_eq!(
            Store::open(dir.path())
                .expect("opens")
                .location()
                .expect("read"),
            None
        );
    }
}
