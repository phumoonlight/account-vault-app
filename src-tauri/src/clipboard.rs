//! Copies text to the system clipboard. Secrets are cleared again after a
//! timeout; anything copied is cleared when the app exits. Both only happen if
//! the clipboard still holds what we copied, so something the user copied
//! afterwards is never wiped.
//!
//! Runs in Rust rather than the webview because the webview can only read the
//! clipboard while the window is focused, and the user is usually pasting
//! into another app by then.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use zeroize::Zeroizing;

/// How long a copied secret stays on the clipboard.
pub const CLEAR_AFTER: Duration = Duration::from_secs(30);

pub trait ClipboardBackend: Send + Sync + 'static {
    fn set_text(&self, text: &str) -> Result<(), String>;
    /// `None` if the clipboard is empty or holds non-text data.
    fn get_text(&self) -> Option<String>;
    fn clear(&self);
}

/// The real OS clipboard. A new handle per call keeps the backend `Sync`.
pub struct SystemClipboard;

/// Windows lets only one process open the clipboard at a time, so an access
/// can fail briefly while another app (clipboard manager, RDP, Office) holds
/// it. Retry a few times rather than silently skipping a clear.
fn with_retry<T>(
    mut op: impl FnMut(&mut arboard::Clipboard) -> Result<T, arboard::Error>,
) -> Result<T, arboard::Error> {
    const ATTEMPTS: u32 = 5;
    let mut attempt = 0;
    loop {
        let result = arboard::Clipboard::new().and_then(|mut c| op(&mut c));
        match result {
            // Empty or non-text clipboard is an answer, not a transient failure.
            Ok(_) | Err(arboard::Error::ContentNotAvailable) => return result,
            Err(_) if attempt + 1 < ATTEMPTS => {
                attempt += 1;
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return result,
        }
    }
}

impl ClipboardBackend for SystemClipboard {
    fn set_text(&self, text: &str) -> Result<(), String> {
        with_retry(|c| c.set_text(text)).map_err(|e| e.to_string())
    }

    fn get_text(&self) -> Option<String> {
        with_retry(|c| c.get_text()).ok()
    }

    fn clear(&self) {
        let _ = with_retry(|c| c.clear());
    }
}

pub struct ClipboardGuard {
    inner: Arc<Inner>,
    clear_after: Duration,
}

struct Inner {
    backend: Box<dyn ClipboardBackend>,
    /// Bumped on every copy, so an older pending clear knows it's stale.
    generation: AtomicU64,
    /// What we last put on the clipboard, if it may still be there.
    last: Mutex<Option<Copied>>,
}

struct Copied {
    text: Zeroizing<String>,
    secret: bool,
}

impl ClipboardGuard {
    pub fn new(backend: impl ClipboardBackend, clear_after: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                backend: Box::new(backend),
                generation: AtomicU64::new(0),
                last: Mutex::new(None),
            }),
            clear_after,
        }
    }

    /// Copies `text`. For a secret, schedules a clear and returns the delay.
    pub fn copy(&self, text: &str, secret: bool) -> Result<Option<Duration>, String> {
        // Held across set + bump so a pending clear can't interleave (see `clear_if_ours`).
        let mut slot = self.inner.last.lock().unwrap();
        self.inner.backend.set_text(text)?;
        let generation = self.inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *slot = Some(Copied {
            text: Zeroizing::new(text.to_owned()),
            secret,
        });
        drop(slot);
        if !secret {
            return Ok(None);
        }

        let inner = Arc::clone(&self.inner);
        let delay = self.clear_after;
        thread::spawn(move || {
            thread::sleep(delay);
            inner.clear_if_ours(Some(generation));
        });
        Ok(Some(delay))
    }

    /// If the clipboard still holds what we last copied, returns whether it
    /// was a secret. Used to warn before closing.
    pub fn still_on_clipboard(&self) -> Option<bool> {
        let slot = self.inner.last.lock().unwrap();
        let copied = slot.as_ref()?;
        self.inner.holds(&copied.text).then_some(copied.secret)
    }

    /// Clears the clipboard now if it still holds what we last copied (on exit).
    pub fn clear_if_ours(&self) {
        self.inner.clear_if_ours(None);
    }
}

impl Inner {
    fn holds(&self, text: &str) -> bool {
        let current = self.backend.get_text().map(Zeroizing::new);
        current.is_some_and(|c| c.as_str() == text)
    }

    /// `timer_generation` is set when called from a secret's timeout: skip if
    /// a newer copy happened since. Checked under the slot lock, which `copy`
    /// also holds, so a newer copy can't slip in between check and clear.
    fn clear_if_ours(&self, timer_generation: Option<u64>) {
        let mut slot = self.last.lock().unwrap();
        if let Some(generation) = timer_generation {
            if self.generation.load(Ordering::SeqCst) != generation {
                return;
            }
        }
        if let Some(copied) = slot.take() {
            if self.holds(&copied.text) {
                self.backend.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct FakeClipboard(Arc<Mutex<Option<String>>>);

    impl FakeClipboard {
        fn get(&self) -> Option<String> {
            self.0.lock().unwrap().clone()
        }
        fn user_copies(&self, text: &str) {
            *self.0.lock().unwrap() = Some(text.into());
        }
    }

    impl ClipboardBackend for FakeClipboard {
        fn set_text(&self, text: &str) -> Result<(), String> {
            self.user_copies(text);
            Ok(())
        }
        fn get_text(&self) -> Option<String> {
            self.get()
        }
        fn clear(&self) {
            *self.0.lock().unwrap() = None;
        }
    }

    const SHORT: Duration = Duration::from_millis(50);
    const WAIT: Duration = Duration::from_millis(200);

    #[test]
    fn secret_is_cleared_after_timeout() {
        let cb = FakeClipboard::default();
        let guard = ClipboardGuard::new(cb.clone(), SHORT);
        assert_eq!(guard.copy("hunter2", true).unwrap(), Some(SHORT));
        assert_eq!(cb.get().as_deref(), Some("hunter2"));
        thread::sleep(WAIT);
        assert_eq!(cb.get(), None);
    }

    #[test]
    fn non_secret_is_not_auto_cleared_but_is_cleared_on_exit() {
        let cb = FakeClipboard::default();
        let guard = ClipboardGuard::new(cb.clone(), SHORT);
        assert_eq!(guard.copy("octocat", false).unwrap(), None);
        thread::sleep(WAIT);
        assert_eq!(cb.get().as_deref(), Some("octocat"));
        assert_eq!(guard.still_on_clipboard(), Some(false));
        guard.clear_if_ours();
        assert_eq!(cb.get(), None);
    }

    #[test]
    fn still_on_clipboard_tracks_what_we_copied() {
        let cb = FakeClipboard::default();
        let guard = ClipboardGuard::new(cb.clone(), Duration::from_secs(60));
        assert_eq!(guard.still_on_clipboard(), None);
        guard.copy("hunter2", true).unwrap();
        assert_eq!(guard.still_on_clipboard(), Some(true));
        cb.user_copies("other");
        assert_eq!(guard.still_on_clipboard(), None);
    }

    #[test]
    fn something_the_user_copied_later_is_not_cleared() {
        let cb = FakeClipboard::default();
        let guard = ClipboardGuard::new(cb.clone(), SHORT);
        guard.copy("hunter2", true).unwrap();
        cb.user_copies("some other text");
        thread::sleep(WAIT);
        guard.clear_if_ours();
        assert_eq!(guard.still_on_clipboard(), None);
        assert_eq!(cb.get().as_deref(), Some("some other text"));
    }

    #[test]
    fn newer_copy_restarts_the_timer() {
        let cb = FakeClipboard::default();
        let guard = ClipboardGuard::new(cb.clone(), Duration::from_millis(150));
        guard.copy("first", true).unwrap();
        thread::sleep(Duration::from_millis(100));
        guard.copy("second", true).unwrap();
        // The first timer fires here but is stale; "second" must survive it.
        thread::sleep(Duration::from_millis(100));
        assert_eq!(cb.get().as_deref(), Some("second"));
        thread::sleep(Duration::from_millis(150));
        assert_eq!(cb.get(), None);
    }

    #[test]
    fn clear_on_exit_only_if_ours() {
        let cb = FakeClipboard::default();
        let guard = ClipboardGuard::new(cb.clone(), Duration::from_secs(60));
        guard.copy("hunter2", true).unwrap();
        guard.clear_if_ours();
        assert_eq!(cb.get(), None);
    }
}
