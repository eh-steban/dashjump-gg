/**
 * Pure selectors for mid-boss UI. Kept outside components so the time/health
 * math can be unit-tested without mounting React. All time arguments are in
 * the same frame as the existing `currentSecond` timeline scrubber -- the
 * backend ships parser tick-derived seconds, and every other overlay
 * (`SinnerLayer`, boss health) compares directly against `currentSecond`.
 */
import {
  FightWindow,
  MidBossData,
  MidBossKillEvent,
  MidBossSpawnEvent,
  RejuvStatusEvent,
} from "../../domain/midBoss";

export interface MidBossCycleState {
  spawn_cycle: number;
  spawn: MidBossSpawnEvent;
  // The kill that ended this cycle, or null if the cycle is still alive
  // (either because the user hasn't scrubbed there yet or the match ended
  // without a kill).
  kill: MidBossKillEvent | null;
}

/** Rejuv grant counts broken down by receiving team. */
export interface RejuvClaimCounts {
  // Team IDs map to Deadlock conventions: 2 = Amber, 3 = Sapphire.
  byTeam: Record<number, number>;
  total: number;
}

/**
 * Finds the spawn cycle whose window contains `currentSecond`. A cycle is
 * "alive" from `spawn_time_s` (inclusive) until its matching kill's
 * `matchtime_s` (exclusive). If no kill exists for the spawn, the cycle stays
 * alive forever (handles match-end-without-kill).
 *
 * Returns null when mid-boss has not yet spawned or the previous cycle has
 * already been killed and no new spawn exists.
 */
export function findActiveCycle(
  midBoss: MidBossData,
  currentSecond: number
): MidBossCycleState | null {
  // Walk spawns in reverse so we pick the most-recent one whose spawn_time_s
  // has already passed. This naturally skips future spawns when scrubbing
  // backwards.
  const spawns = midBoss.spawn_events;
  for (let i = spawns.length - 1; i >= 0; i--) {
    const spawn = spawns[i];
    if (currentSecond < spawn.spawn_time_s) continue;
    const kill =
      midBoss.kill_events.find((k) => k.spawn_cycle === spawn.spawn_cycle) ??
      null;
    if (kill && currentSecond >= kill.matchtime_s) {
      // This cycle already ended before `currentSecond`; the boss is dead.
      return null;
    }
    return { spawn_cycle: spawn.spawn_cycle, spawn, kill };
  }
  return null;
}

/**
 * Returns the mid-boss's current health as an integer, or null when there's
 * nothing to display (no max_health, or no active cycle). Rendering logic:
 *
 * - Before the cycle's first fight window: full health (`max_health`).
 * - Inside a fight window: the most recent sample with `time_s <=
 *   currentSecond`. Step interpolation -- we don't linearly interpolate
 *   between samples, because samples are already emitted at significant state
 *   changes and intermediate ticks carry no new information.
 * - Between fight windows: the previous window's `health_at_end` -- the boss
 *   regenerated no health but wasn't being hit either.
 */
export function currentMidBossHealth(
  midBoss: MidBossData,
  cycle: MidBossCycleState,
  currentSecond: number
): number | null {
  if (midBoss.max_health === null) return null;

  const windowsForCycle = midBoss.fight_windows
    .filter((w) => w.spawn_cycle === cycle.spawn_cycle)
    .sort((a, b) => a.window_start_s - b.window_start_s);

  if (windowsForCycle.length === 0) return midBoss.max_health;

  let lastClosedHealth: number | null = null;
  for (const window of windowsForCycle) {
    if (currentSecond < window.window_start_s) break;
    if (currentSecond <= window.window_end_s) {
      return sampleHealthAt(window, currentSecond);
    }
    lastClosedHealth = window.health_at_end;
  }

  if (lastClosedHealth !== null) return lastClosedHealth;
  return midBoss.max_health;
}

function sampleHealthAt(window: FightWindow, currentSecond: number): number {
  // Samples are already ordered by time_s from the parser. Step interpolation:
  // walk forward while samples still precede `currentSecond`, then stop.
  let current = window.health_at_start;
  for (const sample of window.health_samples) {
    if (sample.time_s > currentSecond) break;
    current = sample.health;
  }
  return current;
}

/**
 * Counts rejuv *grants* (event_type === 6) that belong to a completed kill
 * cycle. Grants are attributed to the nearest preceding kill within a short
 * window -- grants typically fire within a second of the kill.
 *
 * We require `currentSecond >= kill.matchtime_s` so the UI only reveals claim
 * counts after the scrubber has passed the kill, matching how the physical
 * event appears in-game.
 */
export function rejuvClaimsForCompletedKill(
  rejuvEvents: RejuvStatusEvent[],
  kill: MidBossKillEvent,
  currentSecond: number
): RejuvClaimCounts {
  const result: RejuvClaimCounts = { byTeam: {}, total: 0 };
  if (currentSecond < kill.matchtime_s) return result;

  const grantWindowEnd = kill.matchtime_s + REJUV_GRANT_WINDOW_S;
  for (const event of rejuvEvents) {
    if (event.event_type !== 6) continue;
    if (event.matchtime_s < kill.matchtime_s) continue;
    if (event.matchtime_s > grantWindowEnd) continue;
    result.byTeam[event.user_team] = (result.byTeam[event.user_team] ?? 0) + 1;
    result.total += 1;
  }
  return result;
}

// Grants fire within a second or two of the kill in practice; 10s gives a
// generous buffer without risking overlap with the *next* spawn cycle
// (mid-boss respawns are on the order of minutes).
export const REJUV_GRANT_WINDOW_S = 10;
