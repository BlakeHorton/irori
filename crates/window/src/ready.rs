//! Card #232: the window opens before the Core has started, so its first
//! frame can show the opening screen. [`Ready`] is managed when the app is
//! built; the shell sets it once the Core is up (or failed to start), and
//! the frontend's `core_ready` command waits on it before the app mounts.

use std::sync::{Condvar, Mutex};

/// Whether the Core is up: unset while it starts, then `Ok` or the reason
/// it couldn't start.
#[derive(Default)]
pub struct Ready {
    state: Mutex<Option<Result<(), String>>>,
    changed: Condvar,
}

impl Ready {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records how the start ended and wakes every waiter. The first call
    /// wins.
    pub fn set(&self, res: Result<(), String>) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.is_none() {
            *st = Some(res);
            self.changed.notify_all();
        }
    }

    /// Blocks until [`Ready::set`] has been called, and returns its result.
    pub fn wait(&self) -> Result<(), String> {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(res) = st.as_ref() {
                return res.clone();
            }
            st = self.changed.wait(st).unwrap_or_else(|e| e.into_inner());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Ready;
    use std::sync::Arc;

    #[test]
    fn waiters_get_the_first_result() {
        let r = Arc::new(Ready::new());
        let waiter = {
            let r = r.clone();
            std::thread::spawn(move || r.wait())
        };
        r.set(Err("no".into()));
        r.set(Ok(()));
        assert_eq!(waiter.join().unwrap(), Err("no".into()));
        assert_eq!(r.wait(), Err("no".into()));
    }
}
