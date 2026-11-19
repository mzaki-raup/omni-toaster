//! The reminders, as Settings ▸ Reminders sees them
//! (`docs/specs/reminders.md` §10).
//!
//! SQLite is blocking work, so the reminders live on a thread of their own
//! with their own connection, like the image library: the window sends
//! requests and drains replies at frame start, both through **bounded**
//! channels (`.claude/rules/threads-and-scheduling.md`). After every change
//! the thread sends the whole list again, and tells the scheduler, which owns
//! every fire time.

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, SyncSender, TrySendError, sync_channel};

use jiff::Timestamp;
use ot_sched::scheduler::Command;
use ot_store::Store;
use ot_store::reminders::{NotificationAction, Reminder, ReminderDraft, Status};

/// What the window asks.
#[derive(Debug)]
enum Request {
    /// Create (`id` is `None`) or replace a reminder.
    Save {
        id: Option<i64>,
        draft: ReminderDraft,
    },
    Delete(i64),
    History(i64),
    ClearHistory(i64),
    /// Send the list again: a fire may have moved a next time or finished a
    /// reminder.
    Refresh,
}

/// What the thread answers.
#[derive(Debug)]
pub enum Reply {
    Saved(i64),
    Deleted(i64),
    History {
        id: i64,
        rows: Vec<(Timestamp, NotificationAction)>,
    },
    Failed(String),
}

/// Every reminder, and the templates of the built-in types.
#[derive(Debug, Clone, Default)]
pub struct Listing {
    /// Oldest first.
    pub reminders: Vec<Reminder>,
    /// Each built-in type's key and its template's `category` id, in
    /// `ot_store::reminders::REMINDER_TYPES` order.
    pub types: Vec<(&'static str, i64)>,
}

impl Listing {
    /// The type key of a template, if it is a built-in type's.
    pub fn type_of(&self, category_id: Option<i64>) -> Option<&'static str> {
        let category_id = category_id?;
        self.types
            .iter()
            .find(|(_, id)| *id == category_id)
            .map(|(key, _)| *key)
    }

    /// The template of a type key.
    pub fn category_of(&self, type_key: &str) -> Option<i64> {
        self.types
            .iter()
            .find(|(key, _)| *key == type_key)
            .map(|(_, id)| *id)
    }
}

/// The window's side.
pub struct Book {
    requests: SyncSender<Request>,
    replies: Receiver<Reply>,
    lists: Receiver<Listing>,
    /// `None` until the first list arrives: the loading state.
    listing: Option<Listing>,
}

impl Book {
    /// Start the thread on `root`'s database. `scheduler` hears about every
    /// save and delete.
    pub fn new(root: &Path, scheduler: Option<Sender<Command>>) -> Self {
        let (requests, incoming) = sync_channel(16);
        let (reply, replies) = sync_channel(16);
        let (list, lists) = sync_channel(4);
        let root = root.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("reminders".into())
            .spawn(move || serve(&root, &incoming, &reply, &list, scheduler.as_ref()));
        if let Err(error) = spawned {
            tracing::error!(%error, "the reminders thread could not be started");
        }
        Self {
            requests,
            replies,
            lists,
            listing: None,
        }
    }

    /// The reminders, or `None` while they load.
    pub fn listing(&self) -> Option<&Listing> {
        self.listing.as_ref()
    }

    pub fn save(&self, id: Option<i64>, draft: ReminderDraft) {
        self.send(Request::Save { id, draft });
    }

    pub fn delete(&self, id: i64) {
        self.send(Request::Delete(id));
    }

    pub fn history(&self, id: i64) {
        self.send(Request::History(id));
    }

    pub fn clear_history(&self, id: i64) {
        self.send(Request::ClearHistory(id));
    }

    pub fn refresh(&self) {
        self.send(Request::Refresh);
    }

    fn send(&self, request: Request) {
        // Never blocks the UI thread; a full queue is sixteen clicks ahead.
        match self.requests.try_send(request) {
            Ok(()) => {}
            Err(TrySendError::Full(request)) => {
                tracing::warn!(
                    ?request,
                    "the reminders thread is busy; a request was dropped"
                );
            }
            Err(TrySendError::Disconnected(_)) => {
                tracing::warn!("the reminders thread has stopped");
            }
        }
    }

    /// Take the newest list, and the replies. Returns whether a new list
    /// arrived, so the window can rebuild its rows once rather than every
    /// frame.
    pub fn drain(&mut self) -> (bool, Vec<Reply>) {
        let mut fresh = false;
        while let Ok(listing) = self.lists.try_recv() {
            self.listing = Some(listing);
            fresh = true;
        }
        (fresh, self.replies.try_iter().collect())
    }
}

fn serve(
    root: &Path,
    incoming: &Receiver<Request>,
    reply: &SyncSender<Reply>,
    list: &SyncSender<Listing>,
    scheduler: Option<&Sender<Command>>,
) {
    let store = match Store::open(root) {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(%error, "the reminders could not open the database");
            let _ = reply.send(Reply::Failed(error.to_string()));
            let _ = list.send(Listing::default());
            return;
        }
    };
    let tell = |command| {
        if let Some(scheduler) = scheduler {
            let _ = scheduler.send(command);
        }
    };
    let send_list = || match listing(&store) {
        Ok(listing) => {
            let _ = list.send(listing);
        }
        Err(error) => {
            tracing::warn!(%error, "the reminders could not be listed");
            let _ = reply.send(Reply::Failed(error));
        }
    };
    send_list();

    while let Ok(request) = incoming.recv() {
        let answer = match request {
            Request::Save { id, draft } => match save(&store, id, &draft) {
                Ok(id) => {
                    tell(Command::Changed(id));
                    Reply::Saved(id)
                }
                Err(error) => Reply::Failed(error),
            },
            Request::Delete(id) => match store.delete_reminder(id) {
                Ok(_) => {
                    tell(Command::Removed(id));
                    Reply::Deleted(id)
                }
                Err(error) => Reply::Failed(error.to_string()),
            },
            Request::History(id) => match store.history(id) {
                Ok(rows) => Reply::History { id, rows },
                Err(error) => Reply::Failed(error.to_string()),
            },
            Request::ClearHistory(id) => match store.clear_history(id) {
                Ok(_) => Reply::History {
                    id,
                    rows: Vec::new(),
                },
                Err(error) => Reply::Failed(error.to_string()),
            },
            Request::Refresh => {
                send_list();
                continue;
            }
        };
        let _ = reply.send(answer);
        send_list();
    }
}

fn listing(store: &Store) -> Result<Listing, String> {
    let reminders = store.reminders().map_err(|e| e.to_string())?;
    let mut types = Vec::with_capacity(ot_store::reminders::REMINDER_TYPES.len());
    for key in ot_store::reminders::REMINDER_TYPES {
        if let Some(template) = store.template_for_type(key).map_err(|e| e.to_string())? {
            types.push((*key, template.id));
        }
    }
    Ok(Listing { reminders, types })
}

/// Create or replace. An edited reminder is active again: a finished one-off
/// moved to a later time fires, and a waiting snooze is replaced by the new
/// time, which is what the edit asked for.
fn save(store: &Store, id: Option<i64>, draft: &ReminderDraft) -> Result<i64, String> {
    match id {
        None => store.create_reminder(draft).map_err(|e| e.to_string()),
        Some(id) => {
            if !store
                .update_reminder(id, draft)
                .map_err(|e| e.to_string())?
            {
                return Err(format!("reminder {id} is gone"));
            }
            store
                .set_status(id, Status::Active)
                .map_err(|e| e.to_string())?;
            Ok(id)
        }
    }
}

/// The type key of every reminder, for building rows without a lookup each
/// frame.
pub fn types_by_reminder(listing: &Listing) -> HashMap<i64, &'static str> {
    listing
        .reminders
        .iter()
        .filter_map(|r| listing.type_of(r.draft.category_id).map(|key| (r.id, key)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_store::reminders::Trigger;

    fn draft(title: &str) -> ReminderDraft {
        ReminderDraft {
            category_id: None,
            title: title.to_owned(),
            body: None,
            authored_lang: "en-US".to_owned(),
            trigger: Trigger::At {
                at: "2030-01-01T09:00".parse().expect("civil"),
            },
            recurrence: None,
        }
    }

    #[test]
    fn saving_creates_then_replaces_and_reactivates_and_tells_the_scheduler() {
        let dir = tempfile::tempdir().expect("scratch");
        let root = ot_core::DataRoot::resolve(Some(dir.path())).expect("root");
        let (to_scheduler, heard) = std::sync::mpsc::channel();
        let mut book = Book::new(root.path(), Some(to_scheduler));
        // Polls rather than sleeping a guess: the reply, then the list the
        // thread sends after it.
        let settle = |book: &mut Book| {
            let start = std::time::Instant::now();
            let mut replies = Vec::new();
            let mut listed = false;
            while start.elapsed() < std::time::Duration::from_secs(10) {
                let (fresh, more) = book.drain();
                let had_reply = !replies.is_empty();
                replies.extend(more);
                listed |= fresh && had_reply;
                if listed || (!replies.is_empty() && fresh) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            replies
        };

        book.save(None, draft("First"));
        let replies = settle(&mut book);
        let Some(Reply::Saved(id)) = replies.into_iter().next() else {
            panic!("no Saved reply");
        };
        assert_eq!(heard.try_recv(), Ok(Command::Changed(id)));

        // Finished, then edited: active again.
        Store::open(root.path())
            .expect("opens")
            .set_status(id, Status::Done)
            .expect("writes");
        book.save(Some(id), draft("Renamed"));
        let _ = settle(&mut book);
        let listing = book.listing().expect("listed");
        let saved = &listing.reminders[0];
        assert_eq!(saved.draft.title, "Renamed");
        assert_eq!(saved.status, Status::Active);
        assert_eq!(
            listing.types.len(),
            ot_store::reminders::REMINDER_TYPES.len()
        );

        book.delete(id);
        let replies = settle(&mut book);
        assert!(matches!(replies.as_slice(), [.., Reply::Deleted(gone)] if *gone == id));
        assert!(book.listing().expect("listed").reminders.is_empty());
    }
}
