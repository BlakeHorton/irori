//! Agents mentioning agents (card #77; board.md "Agents mention too"): the
//! `mention` arg and `@name` in text, resolved against every device's
//! roster, and the "Mentioned: … Not delivered: …" line.

use coord::mentions::{MISS_AMBIGUOUS, Miss, Resolved};
use coord::{Mention, NOTICE_MENTIONED, Notice};
use inbox::notice::NoticeResult;

use crate::args::Args;
use crate::format::one_line;
use crate::{Caller, Inner, Names};

/// What one mention came to, for the result line.
pub(crate) enum Got {
    Woke,
    Held(String),
    Failed(String),
    Gone,
}

/// The roster as coord's resolver reads it.
fn cards(s: &Inner) -> Vec<coord::roster::AgentCard> {
    s.b.agents()
        .into_iter()
        .map(|a| coord::roster::AgentCard {
            key: a.key,
            device_id: a.device_id,
            name: a.name,
            cwd: a.cwd,
            status: a.status,
            repo_key: a.repo_key,
            ..Default::default()
        })
        .collect()
}

impl Inner {
    /// The caller's mentions in `text` and `given`.
    pub(crate) fn resolve_mentions(&self, c: &Caller, text: &str, given: &[String]) -> Resolved {
        coord::mentions::resolve(&cards(self), &c.sender.key, text, given)
    }

    /// `ms` with each key as card `id`'s log knows it. A key the log
    /// can't map (its agent left) comes back gone.
    pub(crate) fn log_mentions(&self, id: &str, ms: &[Mention]) -> Vec<Mention> {
        ms.iter()
            .map(|m| match self.on(id, std::slice::from_ref(&m.key)) {
                Ok((_, ks)) => Mention {
                    key: ks.into_iter().next().unwrap_or_default(),
                    ..m.clone()
                },
                Err(_) => Mention {
                    gone: true,
                    ..m.clone()
                },
            })
            .collect()
    }

    /// `ms` for card `id`: as its log writes them, and as this device
    /// knows them with the log's gone flags (for the result line).
    pub(crate) fn mentions_on(&self, id: &str, ms: &[Mention]) -> (Vec<Mention>, Vec<Mention>) {
        let logm = self.log_mentions(id, ms);
        let shown = ms
            .iter()
            .zip(&logm)
            .map(|(p, l)| Mention {
                gone: l.gone,
                ..p.clone()
            })
            .collect();
        (logm, shown)
    }

    /// Mentions on card `id` that nothing stores (a blocker's question,
    /// an unblock note): woken, made watchers; the result line.
    pub(crate) fn mention_only(&self, c: &Caller, id: &str, text: &str, a: &Args) -> String {
        let res = self.resolve_mentions(c, text, &a.strs("mention"));
        if res.mentions.is_empty() && res.misses.is_empty() {
            return String::new();
        }
        let (logm, shown) = self.mentions_on(id, &res.mentions);
        let n = self.roster();
        let mres = match self.on(id, std::slice::from_ref(&c.sender.key)) {
            Ok((co, ks)) => match co.mention(&ks[0], id, &logm, text) {
                Ok(Some(notice)) => self.tell(&c.sender, notice),
                Ok(None) => Vec::new(),
                Err(e) => {
                    return format!(
                        "
Not delivered: the mentions ({}).",
                        one_line(&e.to_string(), 160)
                    );
                }
            },
            Err(e) => {
                return format!(
                    "
Not delivered: the mentions ({}).",
                    one_line(&e.to_string(), 160)
                );
            }
        };
        mention_line(&outcomes(&shown, &mres), &res.misses, &n)
    }

    /// Routes `ns` from the caller: the mention notice's results come
    /// back apart from the others'.
    pub(crate) fn tell_all(
        &self,
        c: &Caller,
        ns: Vec<Notice>,
    ) -> (Vec<NoticeResult>, Vec<NoticeResult>) {
        let (mut other, mut mentioned) = (Vec::new(), Vec::new());
        for n in ns {
            let m = n.kind == NOTICE_MENTIONED;
            let rs = self.tell(&c.sender, n);
            if m {
                mentioned.extend(rs);
            } else {
                other.extend(rs);
            }
        }
        (other, mentioned)
    }
}

/// Each of `ms` (private keys) as the mention notice's results left it.
pub(crate) fn outcomes(ms: &[Mention], rs: &[NoticeResult]) -> Vec<(Mention, Got)> {
    ms.iter()
        .map(|m| {
            if m.gone {
                return (m.clone(), Got::Gone);
            }
            let got = match rs.iter().find(|r| r.to == m.key) {
                None => Got::Failed("not sent".into()),
                Some(r) => match &r.err {
                    Some(e) => Got::Failed(e.to_string()),
                    None if r.delivery.state == inbox::st::HELD => {
                        Got::Held(r.delivery.hold.clone())
                    }
                    None => Got::Woke,
                },
            };
            (m.clone(), got)
        })
        .collect()
}

fn named(m: &Mention) -> String {
    let n = one_line(&m.name, 80);
    if n.is_empty() {
        one_line(&m.key, 120)
    } else {
        n
    }
}

fn miss_text(m: &Miss, n: &Names<'_>) -> String {
    let name = one_line(&m.name, 80);
    if m.reason == MISS_AMBIGUOUS {
        let keys: Vec<String> = m
            .candidates
            .iter()
            .map(|k| {
                let cwd = n.cards.get(k).map(|c| c.cwd.clone()).unwrap_or_default();
                if cwd.is_empty() {
                    one_line(k, 120)
                } else {
                    format!("{}, {}", one_line(k, 120), one_line(&cwd, 120))
                }
            })
            .collect();
        return format!(
            "{name} is ambiguous: mention one by key ({})",
            keys.join("; ")
        );
    }
    if m.is_key() {
        return format!("no agent has key {name}");
    }
    format!("\"@{name}\" names no live agent")
}

/// The line a tool's result gains when it had mentions ("" when none).
pub(crate) fn mention_line(got: &[(Mention, Got)], misses: &[Miss], n: &Names<'_>) -> String {
    if got.is_empty() && misses.is_empty() {
        return String::new();
    }
    let (mut ok, mut not) = (Vec::new(), Vec::new());
    for (m, g) in got {
        let who = named(m);
        match g {
            Got::Woke => ok.push(format!("{who} (woke)")),
            Got::Held(h) => ok.push(format!("{who} (held by {})", one_line(h, 80))),
            Got::Failed(e) => not.push(format!("{who}: not sent: {}", one_line(e, 160))),
            Got::Gone => not.push(format!("{who} is gone")),
        }
    }
    not.extend(misses.iter().map(|m| miss_text(m, n)));
    let mut out = String::from("\n");
    if !ok.is_empty() {
        out += &format!("Mentioned: {}.", ok.join(", "));
    }
    if !not.is_empty() {
        if !ok.is_empty() {
            out += " ";
        }
        out += &format!("Not delivered: {}.", not.join("; "));
    }
    out
}

impl Inner {
    /// Sends `m`'s mentioned agents their copies; the result line.
    pub(crate) fn message_mentions(
        &self,
        c: &Caller,
        m: &inbox::Message,
        d: &inbox::Delivery,
        res: &Resolved,
    ) -> String {
        if res.mentions.is_empty() && res.misses.is_empty() {
            return String::new();
        }
        let n = self.roster();
        let to_name = n
            .cards
            .get(&d.to)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let rs = self
            .b
            .inbox()
            .send_mention_copies(&c.sender, m, &d.to, &to_name);
        mention_line(&copy_outcomes(&res.mentions, &rs, &d.to), &res.misses, &n)
    }

    /// `m`'s live mentions watch the card its Ref names on this device's
    /// board.
    pub(crate) fn watch_mentioned(&self, c: &Caller, m: &inbox::Message) {
        let co = self.b.coord();
        let Some(tid) = crate::notice::card_for(co, &m.ref_) else {
            return;
        };
        let who: Vec<String> = m
            .mentions
            .iter()
            .filter(|x| !x.gone)
            .map(|x| x.key.clone())
            .collect();
        if !who.is_empty() {
            let _ = co.watch_for(&c.sender.key, &tid, &who);
        }
    }
}

/// `coord::Mention`s as the inbox stores them.
pub(crate) fn to_inbox(ms: &[Mention]) -> Vec<inbox::Mention> {
    ms.iter()
        .map(|m| inbox::Mention {
            key: m.key.clone(),
            name: m.name.clone(),
            gone: m.gone,
        })
        .collect()
}

/// What a message's copies came to.
pub(crate) fn copy_outcomes(
    ms: &[Mention],
    rs: &[(inbox::Mention, inbox::Result<inbox::Delivery>)],
    skip: &str,
) -> Vec<(Mention, Got)> {
    ms.iter()
        .filter(|m| m.key != skip)
        .map(|m| {
            if m.gone {
                return (m.clone(), Got::Gone);
            }
            let got = match rs.iter().find(|(x, _)| x.key == m.key) {
                None => Got::Failed("not sent".into()),
                Some((_, Err(e))) => Got::Failed(e.to_string()),
                Some((_, Ok(d))) if d.state == inbox::st::HELD => Got::Held(d.hold.clone()),
                Some(_) => Got::Woke,
            };
            (m.clone(), got)
        })
        .collect()
}
