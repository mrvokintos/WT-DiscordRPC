// The current battle: when it started and your kills from the kill feed (/hudmsg).

use crate::killfeed::{self, Event};

#[derive(Debug, Default)]
pub struct Battle {
    pub active: bool,
    /// unix seconds
    pub start: u64,
    pub kills: u32,
    last_id: i64,
    /// the first /hudmsg answer can still hold the previous battle's messages
    skip_history: bool,
}

/// The kill feed name may carry a squadron tag: "[HARBS] mrvokintos (M1A1 SA)".
/// Nicknames are case-sensitive.
fn is_me(unit: &str, me: &str) -> bool {
    !me.is_empty() && (unit.starts_with(&format!("{me} (")) || unit.contains(&format!(" {me} (")))
}

impl Battle {
    /// `in_battle`: a battle map is loaded.
    pub fn tick(&mut self, in_battle: bool, now: u64) {
        if in_battle && !self.active {
            *self = Battle { active: true, start: now, skip_history: true, ..Default::default() };
            log::info!("battle started");
        } else if !in_battle && self.active {
            log::info!("battle over, kills: {}", self.kills);
            self.active = false;
        }
    }

    /// `lastDmg` for the next /hudmsg request.
    pub fn cursor(&self) -> i64 {
        if self.skip_history { 0 } else { self.last_id }
    }

    pub fn on_messages(&mut self, msgs: &[(i64, String)], me: &str) {
        if !self.active {
            return;
        }
        if self.skip_history {
            self.last_id = msgs.iter().map(|m| m.0).max().unwrap_or(0);
            self.skip_history = false;
            return;
        }
        for (id, text) in msgs {
            if *id <= self.last_id {
                continue;
            }
            self.last_id = *id;
            if let Some(Event::Kill { killer, .. }) = killfeed::parse(text) {
                if is_me(&killer, me) {
                    self.kills += 1;
                    log::info!("kill #{}: {text}", self.kills);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_own_kills_after_history() {
        let mut b = Battle::default();
        b.tick(true, 100);
        assert_eq!(b.cursor(), 0);
        b.on_messages(&[(5, "me (T-34) уничтожил X (Tiger)".into())], "me");
        assert_eq!(b.kills, 0, "history is skipped");
        assert_eq!(b.cursor(), 5);
        b.on_messages(
            &[
                (5, "me (T-34) уничтожил X (Tiger)".into()),
                (6, "[TAG] me (T-34) уничтожил Y (Panther)".into()),
                (7, "Z (Tiger) уничтожил me (T-34)".into()),
                (8, "meme (T-34) destroyed Q (M4)".into()),
                (9, "me (Il-2) shot down R (Bf 109)".into()),
            ],
            "me",
        );
        assert_eq!(b.kills, 2);
        b.tick(false, 200);
        b.tick(true, 300);
        assert_eq!((b.kills, b.start), (0, 300));
    }
}
