//! In-memory message hub: an append-only log of messages plus a per-agent
//! read cursor, so each agent only ever sees messages it hasn't read yet.

use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The wildcard recipient: a message addressed to `*` is delivered to every
/// agent that reads its inbox.
pub const BROADCAST: &str = "*";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub from: String,
    /// Recipient agent name, or [`BROADCAST`] for everyone.
    pub to: String,
    /// Arbitrary JSON payload — a string, an object, whatever the agents agree on.
    pub body: serde_json::Value,
    /// Unix milliseconds.
    pub ts: u64,
}

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub body: serde_json::Value,
}

pub struct Hub {
    messages: Vec<Message>,
    next_id: u64,
    /// agent name -> id of the last message delivered to it.
    cursors: HashMap<String, u64>,
    data_path: Option<PathBuf>,
}

impl Hub {
    /// Build a hub, replaying any messages previously persisted to `data_path`.
    pub fn new(data_path: Option<PathBuf>) -> Self {
        let mut hub = Hub {
            messages: Vec::new(),
            next_id: 1,
            cursors: HashMap::new(),
            data_path: data_path.clone(),
        };
        if let Some(path) = &data_path {
            if let Ok(contents) = std::fs::read_to_string(path) {
                for line in contents.lines().filter(|l| !l.trim().is_empty()) {
                    if let Ok(msg) = serde_json::from_str::<Message>(line) {
                        hub.next_id = hub.next_id.max(msg.id + 1);
                        hub.messages.push(msg);
                    }
                }
            }
        }
        hub
    }

    /// Accept a message, assign it an id and timestamp, and persist it.
    pub fn send(&mut self, req: SendRequest) -> Message {
        let msg = Message {
            id: self.next_id,
            from: req.from,
            to: req.to,
            body: req.body,
            ts: now_millis(),
        };
        self.next_id += 1;
        self.persist(&msg);
        self.messages.push(msg.clone());
        msg
    }

    /// Read messages for `agent`. Returns every message addressed to that agent
    /// (or broadcast) newer than its cursor. Unless `peek` is set, the cursor is
    /// advanced so the same messages aren't returned again. `since` overrides the
    /// stored cursor for a one-off read without touching it.
    pub fn inbox(&mut self, agent: &str, since: Option<u64>, peek: bool) -> Vec<Message> {
        let floor = since.unwrap_or_else(|| self.cursors.get(agent).copied().unwrap_or(0));
        let out: Vec<Message> = self
            .messages
            .iter()
            .filter(|m| m.id > floor && (m.to == agent || m.to == BROADCAST))
            .cloned()
            .collect();

        // Advance the cursor to the current high-water mark only for a normal,
        // cursor-based read — a `since` query or a `peek` leaves it untouched.
        if !peek && since.is_none() {
            let high = self.next_id.saturating_sub(1);
            self.cursors.insert(agent.to_string(), high);
        }
        out
    }

    /// All names that have ever sent or received a message (excluding broadcast).
    pub fn agents(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for m in &self.messages {
            set.insert(m.from.clone());
            if m.to != BROADCAST {
                set.insert(m.to.clone());
            }
        }
        set.into_iter().collect()
    }

    pub fn all(&self) -> &[Message] {
        &self.messages
    }

    fn persist(&self, msg: &Message) {
        let Some(path) = &self.data_path else { return };
        let line = match serde_json::to_string(msg) {
            Ok(l) => l,
            Err(_) => return,
        };
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
