//! The last name each agent went by (card #98), so a gone agent, a Grok
//! member after a restart, or another device's agent that left still shows
//! by name instead of its raw key.
//!
//! `agent-names.json` is a JSON object keyed by agent key →
//! `{"name", "at"}` (`at` in Unix seconds, when it was last seen with that
//! name), written through `agent-names.json.tmp`, 0600. Entries older than
//! [`KEEP`] drop.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use coord::roster::AgentCard;
use serde::{Deserialize, Serialize};

use crate::reset::write_private;

pub const FILE: &str = "agent-names.json";
/// A name is kept this long after the agent was last seen with it.
pub const KEEP: Duration = Duration::from_secs(30 * 24 * 3600);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    name: String,
    at: i64,
}

/// Remembers agents' names across their going away and Core restarts.
pub struct AgentNames {
    path: PathBuf,
    m: Mutex<BTreeMap<String, Entry>>,
}

impl AgentNames {
    /// Loads `<data_dir>/agent-names.json`; an unreadable file starts over.
    pub fn new(data_dir: &Path) -> AgentNames {
        let path = data_dir.join(FILE);
        let m = std::fs::read(&path)
            .ok()
            .and_then(|raw| serde_json::from_slice(&raw).ok())
            .unwrap_or_default();
        AgentNames {
            path,
            m: Mutex::new(m),
        }
    }

    /// Records the names on `cards` at Unix time `now`, drops entries past
    /// [`KEEP`], and saves when anything changed. A name seen again within
    /// a day isn't rewritten just to move its time.
    pub fn note(&self, cards: &[AgentCard], now: i64) {
        let mut m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        let mut dirty = false;
        for c in cards {
            let name = c.name.trim();
            if c.key.is_empty() || name.is_empty() {
                continue;
            }
            match m.get_mut(&c.key) {
                Some(e) if e.name == name && now - e.at < 24 * 3600 => {}
                Some(e) => {
                    e.name = name.to_string();
                    e.at = now;
                    dirty = true;
                }
                None => {
                    m.insert(
                        c.key.clone(),
                        Entry {
                            name: name.to_string(),
                            at: now,
                        },
                    );
                    dirty = true;
                }
            }
        }
        let keep = KEEP.as_secs() as i64;
        let before = m.len();
        m.retain(|_, e| now - e.at < keep);
        dirty |= m.len() != before;
        if dirty {
            self.save(&m);
        }
    }

    /// The last name `key` went by, or `""`.
    pub fn get(&self, key: &str) -> String {
        let m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        m.get(key).map(|e| e.name.clone()).unwrap_or_default()
    }

    /// Every remembered key → name.
    pub fn all(&self) -> BTreeMap<String, String> {
        let m = self.m.lock().unwrap_or_else(|e| e.into_inner());
        m.iter().map(|(k, e)| (k.clone(), e.name.clone())).collect()
    }

    fn save(&self, m: &BTreeMap<String, Entry>) {
        let Ok(raw) = serde_json::to_vec_pretty(m) else {
            return;
        };
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        if let Err(e) = write_private(&tmp, &raw) {
            crate::logf(format_args!("agent names: {e}"));
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &self.path) {
            crate::logf(format_args!("agent names: {e}"));
        }
    }
}
