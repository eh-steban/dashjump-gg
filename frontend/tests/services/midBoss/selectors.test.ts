import { describe, expect, it } from 'vitest';
import {
  currentMidBossHealth,
  findActiveCycle,
  rejuvClaimsForCompletedKill,
} from '../../../src/services/midBoss/selectors';
import {
  FightWindow,
  MidBossData,
  MidBossKillEvent,
  MidBossSpawnEvent,
  RejuvStatusEvent,
} from '../../../src/domain/midBoss';

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

const EMPTY_MID_BOSS: MidBossData = {
  boss_name_hash: '16112031173533486177',
  max_health: null,
  spawn_events: [],
  kill_events: [],
  rejuv_events: [],
  fight_windows: [],
  post_match: [],
};

function makeMidBoss(overrides: Partial<MidBossData> = {}): MidBossData {
  return { ...EMPTY_MID_BOSS, ...overrides };
}

function spawn(spawn_cycle: number, spawn_time_s: number): MidBossSpawnEvent {
  return { spawn_cycle, spawn_time_s };
}

function kill(
  spawn_cycle: number,
  matchtime_s: number,
  team = 2,
  team_claimed = 2
): MidBossKillEvent {
  return {
    spawn_cycle,
    team,
    team_claimed,
    matchtime_s,
    x: 0,
    y: 0,
    z: 0,
    bosses_remaining: 0,
  };
}

function window(
  spawn_cycle: number,
  window_start_s: number,
  window_end_s: number,
  health_at_start: number,
  health_at_end: number,
  health_samples: FightWindow['health_samples'] = []
): FightWindow {
  return {
    spawn_cycle,
    window_start_s,
    window_end_s,
    health_at_start,
    health_at_end,
    health_samples,
  };
}

function grant(matchtime_s: number, user_team: number): RejuvStatusEvent {
  return {
    matchtime_s,
    player_pawn: 0,
    user_team,
    killing_team: user_team,
    event_type: 6,
  };
}

// --------------------------------------------------------------------------
// findActiveCycle
// --------------------------------------------------------------------------

describe('findActiveCycle', () => {
  it('returns null before any spawn event', () => {
    const midBoss = makeMidBoss({ spawn_events: [spawn(1, 600)] });
    expect(findActiveCycle(midBoss, 500)).toBeNull();
  });

  it('returns the cycle at the exact spawn time (inclusive lower bound)', () => {
    const midBoss = makeMidBoss({ spawn_events: [spawn(1, 600)] });
    const cycle = findActiveCycle(midBoss, 600);
    expect(cycle?.spawn_cycle).toBe(1);
    expect(cycle?.kill).toBeNull();
  });

  it('returns null at the exact kill time (exclusive upper bound)', () => {
    const midBoss = makeMidBoss({
      spawn_events: [spawn(1, 600)],
      kill_events: [kill(1, 942)],
    });
    expect(findActiveCycle(midBoss, 942)).toBeNull();
  });

  it('returns the cycle when scrubber is inside the window', () => {
    const midBoss = makeMidBoss({
      spawn_events: [spawn(1, 600)],
      kill_events: [kill(1, 942)],
    });
    const cycle = findActiveCycle(midBoss, 800);
    expect(cycle?.spawn_cycle).toBe(1);
    expect(cycle?.kill?.matchtime_s).toBe(942);
  });

  it('stays alive forever when a spawn has no matching kill (match-end case)', () => {
    const midBoss = makeMidBoss({ spawn_events: [spawn(1, 600)] });
    const cycle = findActiveCycle(midBoss, 9999);
    expect(cycle?.spawn_cycle).toBe(1);
    expect(cycle?.kill).toBeNull();
  });

  it('picks the most recent spawn when multiple cycles exist', () => {
    const midBoss = makeMidBoss({
      spawn_events: [spawn(1, 600), spawn(2, 1200)],
      kill_events: [kill(1, 900)],
    });
    const cycle = findActiveCycle(midBoss, 1500);
    expect(cycle?.spawn_cycle).toBe(2);
    expect(cycle?.kill).toBeNull();
  });

  it('reports the dead period between cycles as null', () => {
    const midBoss = makeMidBoss({
      spawn_events: [spawn(1, 600), spawn(2, 1200)],
      kill_events: [kill(1, 900)],
    });
    expect(findActiveCycle(midBoss, 1000)).toBeNull();
  });
});

// --------------------------------------------------------------------------
// currentMidBossHealth
// --------------------------------------------------------------------------

describe('currentMidBossHealth', () => {
  it('returns null when max_health is null', () => {
    const midBoss = makeMidBoss({
      max_health: null,
      spawn_events: [spawn(1, 600)],
    });
    const cycle = findActiveCycle(midBoss, 700)!;
    expect(currentMidBossHealth(midBoss, cycle, 700)).toBeNull();
  });

  it('returns max_health before the first fight window opens', () => {
    const midBoss = makeMidBoss({
      max_health: 14950,
      spawn_events: [spawn(1, 600)],
      fight_windows: [window(1, 800, 900, 14950, 5000)],
    });
    const cycle = findActiveCycle(midBoss, 700)!;
    expect(currentMidBossHealth(midBoss, cycle, 700)).toBe(14950);
  });

  it('uses the most recent sample inside a fight window (step interpolation)', () => {
    const midBoss = makeMidBoss({
      max_health: 14950,
      spawn_events: [spawn(1, 600)],
      fight_windows: [
        window(1, 800, 900, 14950, 5000, [
          { time_s: 810, health: 12000 },
          { time_s: 830, health: 8000 },
          { time_s: 850, health: 6000 },
        ]),
      ],
    });
    const cycle = findActiveCycle(midBoss, 840)!;
    expect(currentMidBossHealth(midBoss, cycle, 840)).toBe(8000);
  });

  it('holds the health_at_end value between fight windows (bait-disengage)', () => {
    const midBoss = makeMidBoss({
      max_health: 14950,
      spawn_events: [spawn(1, 600)],
      fight_windows: [
        window(1, 800, 810, 14950, 13000),
        window(1, 900, 920, 13000, 0),
      ],
    });
    const cycle = findActiveCycle(midBoss, 850)!;
    expect(currentMidBossHealth(midBoss, cycle, 850)).toBe(13000);
  });

  it('applies the sample at exactly window_end_s (inclusive upper bound)', () => {
    // Pins the <= boundary in sampleHealthAt: at 900 we are still inside
    // the window and should reflect the 890 sample, not the post-window
    // health_at_end fallback.
    const midBoss = makeMidBoss({
      max_health: 14950,
      spawn_events: [spawn(1, 600)],
      fight_windows: [
        window(1, 800, 900, 14950, 3000, [
          { time_s: 890, health: 3000 },
        ]),
      ],
    });
    const cycle = findActiveCycle(midBoss, 900)!;
    expect(currentMidBossHealth(midBoss, cycle, 900)).toBe(3000);
  });

  it('uses health_at_start when inside a window but no sample yet passed', () => {
    const midBoss = makeMidBoss({
      max_health: 14950,
      spawn_events: [spawn(1, 600)],
      fight_windows: [
        window(1, 800, 900, 14950, 5000, [{ time_s: 820, health: 12000 }]),
      ],
    });
    const cycle = findActiveCycle(midBoss, 810)!;
    expect(currentMidBossHealth(midBoss, cycle, 810)).toBe(14950);
  });
});

// --------------------------------------------------------------------------
// rejuvClaimsForCompletedKill
// --------------------------------------------------------------------------

describe('rejuvClaimsForCompletedKill', () => {
  it('returns empty counts before currentSecond reaches the kill', () => {
    const k = kill(1, 942);
    const events = [grant(943, 2), grant(943, 2), grant(943, 3)];
    const result = rejuvClaimsForCompletedKill(events, k, 940);
    expect(result.total).toBe(0);
  });

  it('counts a grant that fires at exactly kill.matchtime_s (inclusive lower bound)', () => {
    const k = kill(1, 942);
    const events = [grant(942, 2)];
    const result = rejuvClaimsForCompletedKill(events, k, 945);
    expect(result.total).toBe(1);
    expect(result.byTeam[2]).toBe(1);
  });

  it('counts grants by team after the kill', () => {
    const k = kill(1, 942);
    const events = [grant(943, 2), grant(943, 2), grant(943, 3)];
    const result = rejuvClaimsForCompletedKill(events, k, 945);
    expect(result.total).toBe(3);
    expect(result.byTeam[2]).toBe(2);
    expect(result.byTeam[3]).toBe(1);
  });

  it('ignores non-grant events (consumed, expired)', () => {
    const k = kill(1, 942);
    const events: RejuvStatusEvent[] = [
      grant(943, 2),
      { ...grant(944, 2), event_type: 7 }, // consumed
      { ...grant(945, 3), event_type: 8 }, // expired
    ];
    const result = rejuvClaimsForCompletedKill(events, k, 950);
    expect(result.total).toBe(1);
    expect(result.byTeam[2]).toBe(1);
  });

  it('ignores grants from an earlier kill cycle', () => {
    const k2 = kill(2, 1800);
    const events = [grant(943, 2), grant(1801, 3)];
    const result = rejuvClaimsForCompletedKill(events, k2, 1810);
    expect(result.total).toBe(1);
    expect(result.byTeam[3]).toBe(1);
  });

  it('ignores grants that fire well after the kill window', () => {
    const k = kill(1, 942);
    // A grant 30s later is outside the REJUV_GRANT_WINDOW_S cap and should
    // not be counted -- guards against catching a stray event from a future
    // spawn cycle.
    const events = [grant(975, 2)];
    const result = rejuvClaimsForCompletedKill(events, k, 980);
    expect(result.total).toBe(0);
  });
});
