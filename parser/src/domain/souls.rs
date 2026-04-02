use std::collections::HashMap;

use serde::Serialize;

/// Souls balance for all players at one match second.
///
/// Index into `SoulsData::timeline` with a match-relative second (0 = match start).
#[derive(Debug, Clone, Serialize)]
pub struct SoulsSnapshot {
    /// player_slot -> m_iGoldNetWorth at this second (carry-forward from last known value)
    pub balances: HashMap<u32, i32>,
}

/// A hero kill event recorded during the match.
#[derive(Debug, Clone, Serialize)]
pub struct KillBountyEvent {
    pub match_sec: u32,
    /// entindex_scorer from CCitadelUserMsgHeroKilled
    pub scorer_entindex: i32,
    /// entindex_victim from CCitadelUserMsgHeroKilled
    pub victim_entindex: i32,
}

/// All souls data collected from a single replay.
#[derive(Debug, Clone, Serialize)]
pub struct SoulsData {
    /// Per-second balance snapshots. Index i = match second i (0 = match start).
    pub timeline: Vec<SoulsSnapshot>,
    /// Hero kill events in match-time order.
    pub kill_events: Vec<KillBountyEvent>,
}
