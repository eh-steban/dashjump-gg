import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { page } from 'vitest/browser';
import MidBossHealthBar from '../../../src/components/matchAnalysis/MidBossHealthBar';
import { MidBossData } from '../../../src/domain/midBoss';

const EMPTY_MID_BOSS: MidBossData = {
  boss_name_hash: '16112031173533486177',
  max_health: 14950,
  spawn_events: [],
  kill_events: [],
  rejuv_events: [],
  fight_windows: [],
  post_match: [],
};

function healthBar() {
  return page.getByRole('progressbar', { name: 'Mid-Boss health' }).elements();
}

function rejuvBlock() {
  return page.getByLabelText('Mid-Boss rejuv claims').elements();
}

describe('MidBossHealthBar', () => {
  it('hides the bar before mid-boss spawns', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
    };
    render(<MidBossHealthBar midBoss={midBoss} currentSec={500} />);
    expect(healthBar().length).toBe(0);
  });

  it('shows full health while alive before the first fight window', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
    };
    render(<MidBossHealthBar midBoss={midBoss} currentSec={700} />);
    const [bar] = healthBar();
    expect(bar).toBeDefined();
    expect(bar.getAttribute('aria-valuenow')).toBe('14950');
  });

  it('reflects the current sample inside a fight window', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
      fight_windows: [
        {
          spawn_cycle: 1,
          window_start_s: 800,
          window_end_s: 900,
          health_at_start: 14950,
          health_at_end: 5000,
          health_samples: [
            { time_s: 810, health: 12000 },
            { time_s: 830, health: 8000 },
          ],
        },
      ],
    };
    render(<MidBossHealthBar midBoss={midBoss} currentSec={840} />);
    const [bar] = healthBar();
    expect(bar.getAttribute('aria-valuenow')).toBe('8000');
  });

  it('hides the bar after the kill and shows rejuv claim counts', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
      kill_events: [
        {
          spawn_cycle: 1,
          team: 2,
          team_claimed: 2,
          matchtime_s: 942,
          x: 0,
          y: 0,
          z: 0,
          bosses_remaining: 0,
        },
      ],
      rejuv_events: [
        {
          matchtime_s: 943,
          player_pawn: 0,
          user_team: 2,
          killing_team: 2,
          event_type: 6,
        },
        {
          matchtime_s: 943,
          player_pawn: 0,
          user_team: 2,
          killing_team: 2,
          event_type: 6,
        },
        {
          matchtime_s: 943,
          player_pawn: 0,
          user_team: 3,
          killing_team: 2,
          event_type: 6,
        },
      ],
    };
    render(<MidBossHealthBar midBoss={midBoss} currentSec={945} />);
    expect(healthBar().length).toBe(0);
    const [claims] = rejuvBlock();
    expect(claims).toBeDefined();
    expect(claims.textContent).toContain('Amber: 2');
    expect(claims.textContent).toContain('Sapphire: 1');
  });

  it('renders nothing for a match without mid-boss data', async () => {
    render(<MidBossHealthBar midBoss={EMPTY_MID_BOSS} currentSec={1500} />);
    expect(healthBar().length).toBe(0);
    expect(rejuvBlock().length).toBe(0);
  });

  it('renders nothing when max_health is null even if a cycle would be active', async () => {
    // max_health === null means the boss never spawned in this replay;
    // the whole component (bar + rejuv fallback) should stay silent.
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      max_health: null,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
    };
    render(<MidBossHealthBar midBoss={midBoss} currentSec={700} />);
    expect(healthBar().length).toBe(0);
    expect(rejuvBlock().length).toBe(0);
  });

  it('keeps the rejuv display visible long after the kill (no expiry in v1)', async () => {
    // The claim count freezes at kill time and the RecentClaim fallback
    // shows it indefinitely until the next spawn cycle begins. This test
    // pins that intentional behavior -- a future change that adds an
    // ephemeral display window will need to update it.
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
      kill_events: [
        {
          spawn_cycle: 1,
          team: 2,
          team_claimed: 2,
          matchtime_s: 942,
          x: 0,
          y: 0,
          z: 0,
          bosses_remaining: 0,
        },
      ],
      rejuv_events: [
        {
          matchtime_s: 943,
          player_pawn: 0,
          user_team: 2,
          killing_team: 2,
          event_type: 6,
        },
      ],
    };
    render(<MidBossHealthBar midBoss={midBoss} currentSec={1500} />);
    expect(healthBar().length).toBe(0);
    const [claims] = rejuvBlock();
    expect(claims).toBeDefined();
    expect(claims.textContent).toContain('Amber: 1');
  });
});
