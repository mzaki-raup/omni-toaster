//! Playing a clip, on a thread of its own (FEAT-0060, ADR-022).
//!
//! One thread owns the one output device, opened on the first clip rather than
//! at start, so a machine with no speakers costs nothing until something plays.
//! A new clip replaces the one playing: an Azan is not mixed with anything
//! (`docs/specs/announcements.md` §7). Playback never runs past
//! [`crate::audio::MAX_SECONDS`], and a clip that cannot be opened or played is
//! logged and silent, never a panic.

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use rodio::Source as _;

use crate::audio::MAX_SECONDS;

/// The short chime that ships in the exe: the default for a notification. A
/// stand-in whose licence the Phase 9 terms check must clear
/// (`assets/README.md`).
pub const CHIME: &[u8] = include_bytes!("../../../assets/sound/notification.mp3");
/// The full Azan, for every prayer but Subuh.
pub const AZAN: &[u8] = include_bytes!("../../../assets/sound/azan.mp3");
/// The full Azan for Subuh (Fajr).
pub const AZAN_SUBUH: &[u8] = include_bytes!("../../../assets/sound/azan-subuh.mp3");

/// What to play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    Chime,
    Azan,
    AzanSubuh,
    /// A clip the user imported: a path inside the data folder.
    File(PathBuf),
}

enum Command {
    Play(Clip),
    Stop,
    Quit,
}

/// What the thread tells the rest of the app, without a channel to drain.
#[derive(Debug, Default)]
struct Shared {
    playing: AtomicBool,
    /// The output device could not be opened: nothing will be heard.
    no_device: AtomicBool,
}

/// The audio thread's handle.
pub struct Player {
    commands: Sender<Command>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    /// Start the thread. Opens no device yet.
    pub fn start() -> Self {
        let (commands, incoming) = channel();
        let shared = Arc::new(Shared::default());
        let for_thread = shared.clone();
        let thread = std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || run(&incoming, &for_thread))
            .map_err(|error| tracing::error!(%error, "the audio thread could not be started"))
            .ok();
        Self {
            commands,
            shared,
            thread,
        }
    }

    /// Play `clip`, replacing whatever is playing. Returns at once.
    pub fn play(&self, clip: Clip) {
        let _ = self.commands.send(Command::Play(clip));
    }

    /// Stop what is playing, for a notification that was dismissed.
    pub fn stop(&self) {
        let _ = self.commands.send(Command::Stop);
    }

    /// Whether a clip is playing now.
    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    /// Whether the output device could not be opened.
    pub fn has_no_device(&self) -> bool {
        self.shared.no_device.load(Ordering::Relaxed)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Start `reader` on `sink`, capped at ten minutes.
fn start_reader<R>(
    sink: &rodio::stream::MixerDeviceSink,
    reader: R,
) -> Result<rodio::Player, String>
where
    R: Read + Seek + Send + Sync + 'static,
{
    let decoder = rodio::Decoder::new(reader).map_err(|error| error.to_string())?;
    let player = rodio::Player::connect_new(sink.mixer());
    player.append(decoder.take_duration(Duration::from_secs(MAX_SECONDS)));
    Ok(player)
}

fn open(sink: &rodio::stream::MixerDeviceSink, clip: &Clip) -> Result<rodio::Player, String> {
    match clip {
        Clip::Chime => start_reader(sink, Cursor::new(CHIME)),
        Clip::Azan => start_reader(sink, Cursor::new(AZAN)),
        Clip::AzanSubuh => start_reader(sink, Cursor::new(AZAN_SUBUH)),
        Clip::File(path) => {
            let file = File::open(path).map_err(|error| error.to_string())?;
            start_reader(sink, BufReader::new(file))
        }
    }
}

fn run(incoming: &Receiver<Command>, shared: &Shared) {
    let mut sink: Option<rodio::stream::MixerDeviceSink> = None;
    let mut current: Option<rodio::Player> = None;
    loop {
        match incoming.recv_timeout(Duration::from_millis(200)) {
            Ok(Command::Play(clip)) => {
                if let Some(old) = current.take() {
                    old.stop();
                }
                if sink.is_none() {
                    match rodio::DeviceSinkBuilder::open_default_sink() {
                        Ok(opened) => {
                            shared.no_device.store(false, Ordering::Relaxed);
                            sink = Some(opened);
                        }
                        Err(error) => {
                            shared.no_device.store(true, Ordering::Relaxed);
                            tracing::warn!(%error, "no audio output could be opened");
                        }
                    }
                }
                if let Some(sink) = &sink {
                    match open(sink, &clip) {
                        Ok(player) => {
                            tracing::info!(?clip, "playing a clip");
                            current = Some(player);
                        }
                        Err(error) => tracing::warn!(?clip, %error, "a clip could not be played"),
                    }
                }
            }
            Ok(Command::Stop) => {
                if let Some(old) = current.take() {
                    old.stop();
                    tracing::info!("the clip was stopped");
                }
            }
            Ok(Command::Quit) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        shared.playing.store(
            current.as_ref().is_some_and(|player| !player.empty()),
            Ordering::Relaxed,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio;
    use std::time::Instant;

    fn wait_for(what: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if what() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        what()
    }

    #[test]
    fn the_clips_in_the_exe_are_real_audio_within_the_cap() {
        for (name, clip) in [("chime", CHIME), ("azan", AZAN), ("azan-subuh", AZAN_SUBUH)] {
            let checked = audio::check(clip).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(checked.format, audio::Format::Mp3, "{name}");
            assert!(
                checked.duration.as_secs() >= 1,
                "{name}: {:?}",
                checked.duration
            );
        }
    }

    #[test]
    fn a_clip_that_cannot_be_opened_is_silent_not_a_panic_and_stop_is_safe_at_any_time() {
        let player = Player::start();
        player.stop(); // nothing is playing
        player.play(Clip::File(PathBuf::from("D:/no/such/clip.mp3")));
        // Whatever the machine has — a device or none — this must come back to
        // not playing, with the thread alive to take the next command.
        std::thread::sleep(Duration::from_millis(400));
        assert!(!player.is_playing(), "a missing file plays nothing");
        player.stop();
    }

    #[test]
    fn a_clip_starts_and_stops_or_the_machine_has_no_output_and_says_so() {
        let player = Player::start();
        player.play(Clip::Chime);
        // Either it plays, or there is no device and the player reports it:
        // never a silent third state.
        let settled = wait_for(|| player.is_playing() || player.has_no_device());
        assert!(settled, "neither playing nor reporting no device");
        if player.is_playing() {
            player.stop();
            assert!(wait_for(|| !player.is_playing()), "stop did not stop it");
        }
    }
}
