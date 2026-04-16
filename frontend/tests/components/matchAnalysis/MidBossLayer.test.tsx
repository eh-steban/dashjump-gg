import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { page } from 'vitest/browser';
import MidBossLayer, {
  ICON_SIZE,
} from '../../../src/components/matchAnalysis/MidBossLayer';
import { MidBossData } from '../../../src/domain/midBoss';

// Minimum-viable world -> pixel projection for tests. Returns a fixed point
// so we can assert the centering math directly.
const identityProjection = () => ({ left: 100, top: 100 });

const EMPTY_MID_BOSS: MidBossData = {
  boss_name_hash: '16112031173533486177',
  max_health: 14950,
  spawn_events: [],
  kill_events: [],
  rejuv_events: [],
  fight_windows: [],
  post_match: [],
};

function midBossIcons() {
  return page.getByRole('img', { name: 'Mid-Boss' }).elements();
}

describe('MidBossLayer', () => {
  it('renders nothing before the first spawn event', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
    };
    render(
      <MidBossLayer
        midBoss={midBoss}
        currentSec={500}
        worldToMinimapPixels={identityProjection}
      />
    );
    expect(midBossIcons().length).toBe(0);
  });

  it('renders the icon while the boss is alive', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
      kill_events: [
        {
          spawn_cycle: 1,
          team_killed: 2,
          team_claimed: 2,
          rejuvs_by_team: { "2": 0, "3": 0 },
          matchtime_s: 942,
          x: 0,
          y: 0,
          z: 0,
          bosses_remaining: 0,
        },
      ],
    };
    render(
      <MidBossLayer
        midBoss={midBoss}
        currentSec={800}
        worldToMinimapPixels={identityProjection}
      />
    );
    expect(midBossIcons().length).toBe(1);
  });

  it('centers the icon at (left, top) minus half its size', async () => {
    // Regression guard: catches off-by-half-size centering math.
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
    };
    render(
      <MidBossLayer
        midBoss={midBoss}
        currentSec={800}
        worldToMinimapPixels={() => ({ left: 200, top: 300 })}
      />
    );
    const [icon] = midBossIcons() as HTMLImageElement[];
    expect(icon).toBeDefined();
    expect(icon.style.left).toBe(`${200 - ICON_SIZE / 2}px`);
    expect(icon.style.top).toBe(`${300 - ICON_SIZE / 2}px`);
    expect(icon.style.width).toBe(`${ICON_SIZE}px`);
    expect(icon.style.height).toBe(`${ICON_SIZE}px`);
  });

  it('removes the icon at the kill time (exclusive upper bound)', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
      kill_events: [
        {
          spawn_cycle: 1,
          team_killed: 2,
          team_claimed: 2,
          rejuvs_by_team: { "2": 0, "3": 0 },
          matchtime_s: 942,
          x: 0,
          y: 0,
          z: 0,
          bosses_remaining: 0,
        },
      ],
    };
    render(
      <MidBossLayer
        midBoss={midBoss}
        currentSec={942}
        worldToMinimapPixels={identityProjection}
      />
    );
    expect(midBossIcons().length).toBe(0);
  });

  it('keeps the icon visible when the match ends without a kill', async () => {
    const midBoss: MidBossData = {
      ...EMPTY_MID_BOSS,
      spawn_events: [{ spawn_cycle: 1, spawn_time_s: 600 }],
    };
    render(
      <MidBossLayer
        midBoss={midBoss}
        currentSec={3600}
        worldToMinimapPixels={identityProjection}
      />
    );
    expect(midBossIcons().length).toBe(1);
  });

  it('renders nothing for a match where the mid-boss never spawned', async () => {
    render(
      <MidBossLayer
        midBoss={EMPTY_MID_BOSS}
        currentSec={1500}
        worldToMinimapPixels={identityProjection}
      />
    );
    expect(midBossIcons().length).toBe(0);
  });
});
