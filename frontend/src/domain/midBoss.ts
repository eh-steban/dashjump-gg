// Mid-boss types mirror the backend `MidBossData` contract documented in
// `private/specs/contracts/backend-api.md` and `parser-api.md`. Field names
// are snake_case because the backend ships JSON as-is.

export interface MidBossSpawnEvent {
  spawn_cycle: number;
  spawn_time_s: number;
  // Per-cycle scaling: Deadlock rescales mid-boss max health each spawn
  // (approx 13000 + 195 * match_minutes). Always read from the active spawn
  // event -- there is no match-global max_health.
  max_health: number;
}

export interface MidBossKillEvent {
  spawn_cycle: number;
  team_killed: number;
  team_claimed: number;
  rejuvs_by_team: Record<string, number>;
  matchtime_s: number;
  x: number;
  y: number;
  z: number;
  bosses_remaining: number;
}

export interface RejuvStatusEvent {
  matchtime_s: number;
  player_pawn: number;
  user_team: number;
  killing_team: number;
  // 6 = buff granted, 7 = buff consumed (revive), 8 = buff expired
  event_type: number;
}

export interface HealthSample {
  time_s: number;
  health: number;
}

export interface FightWindow {
  spawn_cycle: number;
  window_start_s: number;
  window_end_s: number;
  health_at_start: number;
  health_at_end: number;
  health_samples: HealthSample[];
}

// Parser-derived summary of each mid-boss kill cycle. `team_claimed` is
// intentionally absent here -- callers who want the "who won this fight"
// verdict should read it off `MidBossKillEvent` (which applies strict-majority
// derivation). Valve's raw blob still lives at `match_metadata.match_info.mid_boss`.
export interface MidBossPostMatch {
  team_killed: number;
  rejuvs_by_team: Record<string, number>;
  destroyed_time_s: number;
}

export interface MidBossData {
  boss_name_hash: string;
  spawn_events: MidBossSpawnEvent[];
  kill_events: MidBossKillEvent[];
  rejuv_events: RejuvStatusEvent[];
  fight_windows: FightWindow[];
  post_match: MidBossPostMatch[];
}
