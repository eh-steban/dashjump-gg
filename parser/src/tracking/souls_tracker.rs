use std::collections::HashMap;

use crate::domain::{KillBountyEvent, SoulsData, SoulsSnapshot};

/// Tracks per-player souls balances and hero kill events throughout a match.
#[derive(Debug)]
pub struct SoulsTracker {
    /// entity_index -> (player_slot, balance) -- updated on every CCitadelPlayerPawn entity event
    current_balances: HashMap<i32, (u32, i32)>,
    timeline: Vec<SoulsSnapshot>,
    kill_events: Vec<KillBountyEvent>,
}

impl SoulsTracker {
    pub fn new() -> Self {
        Self {
            current_balances: HashMap::new(),
            timeline: Vec::new(),
            kill_events: Vec::new(),
        }
    }

    /// Update balance for an entity. Called from on_entity for CCitadelPlayerPawn.
    pub fn handle_pawn_update(&mut self, entity_index: i32, player_slot: u32, balance: i32) {
        self.current_balances.insert(entity_index, (player_slot, balance));
    }

    /// Emit one snapshot for `match_sec`. Carries forward all current balances -- no reset per tick.
    ///
    /// Called from on_tick_end once per match second alongside existing snapshot calls.
    pub fn build_snapshot(&mut self, match_sec: u32) {
        // Extend timeline to cover any gap between last snapshot and this second.
        // Gaps can occur if build_snapshot is skipped for a second (e.g., at match start).
        let needed_len = (match_sec + 1) as usize;
        while self.timeline.len() < needed_len.saturating_sub(1) {
            let carry = self.make_balances_map();
            self.timeline.push(SoulsSnapshot { balances: carry });
        }

        let balances = self.make_balances_map();
        self.timeline.push(SoulsSnapshot { balances });
    }

    fn make_balances_map(&self) -> HashMap<u32, i32> {
        self.current_balances
            .values()
            .map(|&(slot, balance)| (slot, balance))
            .collect()
    }

    /// Record a hero kill event. Called from on_packet for KEUserMsgHeroKilled.
    pub fn handle_hero_killed(
        &mut self,
        scorer_entindex: i32,
        victim_entindex: i32,
        match_sec: u32,
    ) {
        self.kill_events.push(KillBountyEvent {
            match_sec,
            scorer_entindex,
            victim_entindex,
        });
    }

    /// Return all collected data.
    pub fn get_output(&self) -> SoulsData {
        SoulsData {
            timeline: self.timeline.clone(),
            kill_events: self.kill_events.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
