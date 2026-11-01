//! Stop and Pause (card #207, docs/design/stop-pause.md): the person cuts a
//! busy agent's running turn, or pauses a card's holders until Resume.

use crate::Window;

/// What a Stop says when the person wrote nothing.
pub const STOP_DEFAULT: &str = "Stop and wait for me.";

impl Window {
    /// AgentService.Stop: cuts `key`'s running turn; `text` (or
    /// [`STOP_DEFAULT`]) is its next prompt.
    pub fn agent_stop(&self, key: &str, text: &str) -> Result<(), String> {
        let key = crate::trim(key);
        if key.is_empty() {
            return Err("stop whom? no agent key".into());
        }
        let text = match crate::trim(text) {
            "" => STOP_DEFAULT,
            t => t,
        };
        self.core
            .inbox()
            .send_with(
                &self.core.user_sender(),
                &::inbox::Address::new(::inbox::addr::AGENT, &self.core.private_key(key)),
                text,
                ::inbox::SendOptions {
                    mode: ::inbox::mode::STOP.into(),
                    ..::inbox::SendOptions::default()
                },
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// BoardService.Pause: pauses the card `id` (or resumes it). Pausing
    /// stops each holder with the person's note; resuming wakes them. Only
    /// the person's own board's cards. Returns who was told.
    pub fn pause(&self, id: &str, paused: bool, note: &str) -> Result<Vec<String>, String> {
        let me = self.core.user_sender();
        let (t, changed) = self
            .core
            .coord()
            .pause(&me.key, crate::trim(id), paused, note)
            .map_err(|e| e.to_string())?;
        if !changed {
            return Ok(Vec::new());
        }
        let r = t.short_ref();
        let who = if me.name.trim().is_empty() {
            "The person".to_string()
        } else {
            me.name.trim().to_string()
        };
        let note = crate::trim(note);
        let text = if paused {
            let mut s = format!(
                "{who} paused {r} \"{}\". Your tools are off for this card except Irori's own: answer {who} on the card (board_comment) and wait.",
                t.title
            );
            if !note.is_empty() {
                s.push_str("\n\n");
                s.push_str(note);
            }
            s
        } else {
            format!("{who} resumed {r} \"{}\": carry on.", t.title)
        };
        let mut keys: Vec<String> = Vec::new();
        for k in t.holders.iter().chain(t.co_holders.iter()) {
            if !k.is_empty() && !keys.contains(k) {
                keys.push(k.clone());
            }
        }
        let ib = self.core.inbox();
        let (mut told, mut errs) = (Vec::new(), Vec::new());
        for k in &keys {
            let to = self.core.private_key(k);
            let o = ::inbox::SendOptions {
                mode: if paused {
                    ::inbox::mode::STOP.into()
                } else {
                    ::inbox::mode::WAKE.into()
                },
                reason: if paused {
                    ::inbox::REASON_PAUSED.into()
                } else {
                    ::inbox::REASON_RESUMED.into()
                },
                ref_: r.clone(),
                ..::inbox::SendOptions::default()
            };
            match ib.send_with(&me, &::inbox::Address::new(::inbox::addr::AGENT, &to), &text, o) {
                Ok(_) => told.push(to),
                Err(e) => errs.push(format!("{to}: {e}")),
            }
        }
        if errs.is_empty() {
            Ok(told)
        } else {
            Err(format!(
                "{r} is {}, but not every holder heard: {}",
                if paused { "paused" } else { "resumed" },
                errs.join("; ")
            ))
        }
    }
}
