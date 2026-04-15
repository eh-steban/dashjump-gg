// Mid-boss types mirror the backend `MidBossData` contract documented in
// `private/specs/contracts/backend-api.md` and `parser-output.md`. Field names
// are snake_case because the backend ships JSON as-is.

export interface MidBossSpawnEvent {
  spawn_cycle: number;
  spawn_time_s: number;
}

export interface MidBossKillEvent {
  spawn_cycle: number;
  team: number;
  team_claimed: number;
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

// Mirrors Valve's post-match mid_boss blob. Logged to the console for
// cross-validation with our replay-derived team_claimed; not rendered in UI.
export interface MidBossPostMatch {
  team_killed: number;
  team_claimed: number;
  destroyed_time_s: number;
}

export interface MidBossData {
  boss_name_hash: string;
  max_health: number | null;
  spawn_events: MidBossSpawnEvent[];
  kill_events: MidBossKillEvent[];
  rejuv_events: RejuvStatusEvent[];
  fight_windows: FightWindow[];
  post_match: MidBossPostMatch[];
}
