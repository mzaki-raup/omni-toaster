//! The pictures that ship inside the exe (`docs/specs/toast.md` §8.2).
//!
//! Prayer times, weather and reminder types each get a default picture
//! without the user importing anything. They are **embedded**, not copied into
//! `OmniToasterData/`: the portable folder stays the user's own, nothing is
//! written on first run, and a picture can never go missing from under a
//! template that names it.
//!
//! A built-in picture is named by a key, `@builtin/<name>`, which stands where
//! a library picture's relative path would — the same device as the app's own
//! icon's `@app-icon`. The `@` cannot begin an imported file's relative path,
//! which always starts `assets/`, so the two can never collide, and the key
//! only ever resolves against the table below, never against the disk.
//!
//! Each file is a 256 × 256 WebP, downscaled from the 512 × 512 originals: a
//! toast shows a picture at about 72 points, so the full size would only make
//! the exe bigger. `a_builtin_picture_is_small_and_decodes` holds the budget.

/// What a built-in picture is for, which is how the pickers group them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Prayer,
    Weather,
    Reminder,
}

impl Group {
    pub fn all() -> [Self; 3] {
        [Self::Prayer, Self::Weather, Self::Reminder]
    }

    /// The Fluent key naming the group in a picker.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Prayer => "settings-builtin-group-prayer",
            Self::Weather => "settings-builtin-group-weather",
            Self::Reminder => "settings-builtin-group-reminder",
        }
    }
}

/// One picture that ships with the app.
#[derive(Debug, Clone, Copy)]
pub struct Builtin {
    /// The file's name without its extension, which is also the key's tail.
    pub name: &'static str,
    /// The key a template or a toast stores: `@builtin/<name>`.
    pub key: &'static str,
    pub group: Group,
    /// The Fluent key for the picture's name. Never the name itself: the
    /// label is shown in the user's language.
    pub label_key: &'static str,
    /// The WebP file, as embedded.
    pub bytes: &'static [u8],
}

/// The prefix of every built-in key.
pub const PREFIX: &str = "@builtin/";

macro_rules! picture {
    ($name:literal, $group:expr) => {
        Builtin {
            name: $name,
            key: concat!("@builtin/", $name),
            group: $group,
            label_key: concat!("settings-builtin-", $name),
            // A file that is not there is a compile error, not a blank.
            bytes: include_bytes!(concat!("../../../assets/builtin/pictures/", $name, ".webp")),
        }
    };
}

/// Every built-in picture, in the order the pickers list them: the day's
/// prayers in the order they fall, then the weather from clear to stormy,
/// then the reminder types.
static PICTURES: &[Builtin] = &[
    picture!("masjid-imsak", Group::Prayer),
    picture!("masjid-subuh", Group::Prayer),
    picture!("masjid-dhuha", Group::Prayer),
    picture!("masjid-zohor", Group::Prayer),
    picture!("masjid-asar", Group::Prayer),
    picture!("masjid-maghrib", Group::Prayer),
    picture!("masjid-isyak", Group::Prayer),
    picture!("sunny", Group::Weather),
    picture!("partly-cloudy", Group::Weather),
    picture!("cloudy", Group::Weather),
    picture!("hazy", Group::Weather),
    picture!("fog", Group::Weather),
    picture!("drizzle", Group::Weather),
    picture!("downpour", Group::Weather),
    picture!("thunderstorm", Group::Weather),
    picture!("windy", Group::Weather),
    picture!("alarm-clock", Group::Reminder),
    picture!("meeting", Group::Reminder),
    picture!("training", Group::Reminder),
];

/// Every built-in picture.
pub fn all() -> &'static [Builtin] {
    PICTURES
}

/// The built-in picture a key names, or `None` for anything else — including
/// a key from a newer build that this one does not have, which the caller
/// treats as no picture rather than an error.
pub fn find(key: &str) -> Option<&'static Builtin> {
    key.strip_prefix(PREFIX)?;
    PICTURES.iter().find(|picture| picture.key == key)
}

/// The built-in pictures of one group, in listing order.
pub fn in_group(group: Group) -> impl Iterator<Item = &'static Builtin> {
    PICTURES
        .iter()
        .filter(move |picture| picture.group == group)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::assets::{self, Format};

    /// What the pictures may add to the exe, in bytes. Nineteen at about 17 KB
    /// is 330 KB; this leaves room for a few more and stops a 512 px original
    /// being dropped in unshrunk.
    const BUDGET: usize = 600 * 1024;

    #[test]
    fn every_key_is_unique_prefixed_and_findable() {
        let mut seen = std::collections::BTreeSet::new();
        for picture in all() {
            assert_eq!(picture.key, format!("{PREFIX}{}", picture.name));
            assert!(seen.insert(picture.key), "{} is listed twice", picture.key);
            assert_eq!(
                find(picture.key).map(|found| found.name),
                Some(picture.name)
            );
        }
    }

    #[test]
    fn what_is_not_a_builtin_key_finds_nothing() {
        assert!(find("@builtin/no-such-picture").is_none());
        assert!(find("assets/0423dede.png").is_none());
        assert!(find("@app-icon").is_none());
        assert!(find("").is_none());
        // The prefix alone, and a path dressed as a key, name nothing.
        assert!(find("@builtin/").is_none());
        assert!(find("@builtin/../Cargo.toml").is_none());
    }

    #[test]
    fn a_builtin_picture_is_small_and_decodes() {
        let mut total = 0;
        for picture in all() {
            let path = Path::new(picture.key);
            assert_eq!(
                assets::sniff(picture.bytes),
                Some(Format::Webp),
                "{} is not a WebP",
                picture.name
            );
            let decoded = assets::decode(picture.bytes, path, 256)
                .unwrap_or_else(|error| panic!("{} did not decode: {error}", picture.name));
            assert!(
                decoded.width <= 256 && decoded.height <= 256,
                "{} is {}x{}: ship it shrunk to 256",
                picture.name,
                decoded.width,
                decoded.height
            );
            assert_eq!(
                decoded.rgba.len(),
                (decoded.width * decoded.height * 4) as usize
            );
            total += picture.bytes.len();
        }
        assert!(
            total <= BUDGET,
            "the pictures weigh {total} bytes; the budget is {BUDGET}"
        );
    }

    #[test]
    fn the_weather_icons_keep_their_transparency() {
        // They are drawn on whatever the toast's background is. A conversion
        // that flattened them onto white would show as a white square.
        let sunny = find("@builtin/sunny").expect("sunny");
        let decoded = assets::decode(sunny.bytes, Path::new(sunny.key), 256).expect("decodes");
        let corner_alpha = decoded.rgba[3];
        assert_eq!(corner_alpha, 0, "the corner of the sun is not transparent");
    }

    #[test]
    fn the_groups_hold_what_their_names_say() {
        let names = |group| in_group(group).map(|p| p.name).collect::<Vec<_>>();
        assert!(
            names(Group::Prayer)
                .iter()
                .all(|n| n.starts_with("masjid-"))
        );
        assert_eq!(names(Group::Prayer).len(), 7);
        assert!(names(Group::Weather).contains(&"thunderstorm"));
        assert!(names(Group::Reminder).contains(&"alarm-clock"));
        assert_eq!(
            Group::all().iter().map(|g| names(*g).len()).sum::<usize>(),
            all().len(),
            "every picture is in exactly one group"
        );
    }
}
