import { describe, expect, it, beforeEach } from 'vitest';
import { render } from 'vitest-browser-react';
import { page } from 'vitest/browser';
import CreepWaveLayer, {
  isPinVisible,
} from '../../../src/components/matchAnalysis/CreepWaveLayer';
import { LaneCreepData, WaveMeta } from '../../../src/domain/creep';

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

const makeWorldToMinimap =
  (scale = 1) =>
  (x: number, y: number) => ({
    left: x * scale,
    top: y * scale,
  });

const EMPTY_DATA: LaneCreepData = { creeps: {}, wave_meta: {} };

// --------------------------------------------------------------------------
// Unit tests: isPinVisible
// --------------------------------------------------------------------------

describe('isPinVisible', () => {
  const allMeta: Record<string, WaveMeta> = {
    '1_2_45': {
      lane: 1,
      team: 2,
      spawn_sec: 45,
      last_death_sec: 65,
      last_death_x: 1500,
      last_death_y: 2300,
    },
    '1_2_90': {
      lane: 1,
      team: 2,
      spawn_sec: 90,
      last_death_sec: null,
      last_death_x: null,
      last_death_y: null,
    },
  };

  it('returns false when last_death_sec is null', () => {
    expect(isPinVisible('1_2_90', allMeta['1_2_90'], 200, allMeta)).toBe(false);
  });

  it('returns false when currentSec is before last_death_sec', () => {
    expect(isPinVisible('1_2_45', allMeta['1_2_45'], 60, allMeta)).toBe(false);
  });

  it('returns true when currentSec equals last_death_sec and no newer dead wave', () => {
    expect(isPinVisible('1_2_45', allMeta['1_2_45'], 65, allMeta)).toBe(true);
  });

  it('returns true after last_death_sec when no newer dead wave exists', () => {
    expect(isPinVisible('1_2_45', allMeta['1_2_45'], 100, allMeta)).toBe(true);
  });

  it('returns false when a newer wave on same lane+team has died by currentSec', () => {
    const metaWithNewerDead: Record<string, WaveMeta> = {
      '1_2_45': {
        lane: 1,
        team: 2,
        spawn_sec: 45,
        last_death_sec: 65,
        last_death_x: 1500,
        last_death_y: 2300,
      },
      '1_2_90': {
        lane: 1,
        team: 2,
        spawn_sec: 90,
        last_death_sec: 110,
        last_death_x: 1600,
        last_death_y: 2400,
      },
    };
    // At sec 120, newer wave '1_2_90' (spawn 90, death 110) supersedes '1_2_45'
    expect(
      isPinVisible('1_2_45', metaWithNewerDead['1_2_45'], 120, metaWithNewerDead)
    ).toBe(false);
  });

  it('does not suppress a wave using a different lane', () => {
    const crossLaneMeta: Record<string, WaveMeta> = {
      '1_2_45': {
        lane: 1,
        team: 2,
        spawn_sec: 45,
        last_death_sec: 65,
        last_death_x: 1500,
        last_death_y: 2300,
      },
      '2_2_90': {
        lane: 2,
        team: 2,
        spawn_sec: 90,
        last_death_sec: 110,
        last_death_x: 2000,
        last_death_y: 2400,
      },
    };
    // Lane 2 wave should not suppress lane 1 wave
    expect(
      isPinVisible('1_2_45', crossLaneMeta['1_2_45'], 120, crossLaneMeta)
    ).toBe(true);
  });

  it('does not suppress a wave using a different team', () => {
    const crossTeamMeta: Record<string, WaveMeta> = {
      '1_2_45': {
        lane: 1,
        team: 2,
        spawn_sec: 45,
        last_death_sec: 65,
        last_death_x: 1500,
        last_death_y: 2300,
      },
      '1_3_90': {
        lane: 1,
        team: 3,
        spawn_sec: 90,
        last_death_sec: 110,
        last_death_x: 2000,
        last_death_y: 2400,
      },
    };
    expect(
      isPinVisible('1_2_45', crossTeamMeta['1_2_45'], 120, crossTeamMeta)
    ).toBe(true);
  });
});

// --------------------------------------------------------------------------
// Snapshot tests: CreepWaveLayer renders dots at correct pixel positions
// --------------------------------------------------------------------------

describe('CreepWaveLayer', () => {
  it('renders nothing when data is empty', async () => {
    render(
      <CreepWaveLayer
        laneCreepData={EMPTY_DATA}
        currentSec={0}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    // No creep or pin elements should be present
    const svgCircles = page.getByRole('img');
    await expect.element(svgCircles).not.toBeInTheDocument();
  });

  it('renders a live creep dot for each alive creep at currentSec', async () => {
    const data: LaneCreepData = {
      creeps: {
        '42': [
          null,
          { x: 100, y: 200, lane: 1, team: 2, wave_id: '1_2_45', nearby_players: [] },
          null,
        ],
        '43': [
          null,
          { x: 300, y: 400, lane: 1, team: 2, wave_id: '1_2_45', nearby_players: [3] },
          null,
        ],
      },
      wave_meta: {
        '1_2_45': {
          lane: 1,
          team: 2,
          spawn_sec: 45,
          last_death_sec: null,
          last_death_x: null,
          last_death_y: null,
        },
      },
    };

    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={1}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    // Two creep indicators should be in the DOM
    const containers = page.getByRole('presentation');
    // We test via the SVG circles — each CreepWaveIndicator renders one <svg>
    const svgElements = document.querySelectorAll('svg');
    expect(svgElements.length).toBe(2);
  });

  it('positions creep dot at the correct pixel coordinate', async () => {
    const data: LaneCreepData = {
      creeps: {
        '42': [
          { x: 50, y: 75, lane: 1, team: 3, wave_id: '1_3_30', nearby_players: [] },
        ],
      },
      wave_meta: {},
    };

    const scale = 2;
    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={0}
        worldToMinimapPixels={makeWorldToMinimap(scale)}
      />
    );

    // Expected pixel position: x*2=100, y*2=150
    const indicator = document.querySelector('div[style*="left: 100px"]');
    expect(indicator).not.toBeNull();
    const topIndicator = document.querySelector('div[style*="top: 150px"]');
    expect(topIndicator).not.toBeNull();
  });

  it('does not render a creep dot when snapshot at currentSec is null', async () => {
    const data: LaneCreepData = {
      creeps: {
        '42': [null, null, null],
      },
      wave_meta: {},
    };

    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={1}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    const svgElements = document.querySelectorAll('svg');
    expect(svgElements.length).toBe(0);
  });

  it('renders a pin at last_death position after wave dies', async () => {
    const data: LaneCreepData = {
      creeps: {},
      wave_meta: {
        '1_2_45': {
          lane: 1,
          team: 2,
          spawn_sec: 45,
          last_death_sec: 65,
          last_death_x: 1500,
          last_death_y: 2300,
        },
      },
    };

    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={70}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    // One pin indicator rendered
    const svgElements = document.querySelectorAll('svg');
    expect(svgElements.length).toBe(1);

    // Pin circle fill should be black
    const circle = document.querySelector('circle');
    expect(circle?.getAttribute('fill')).toBe('#000000');
  });

  it('does not render a pin before wave last_death_sec', async () => {
    const data: LaneCreepData = {
      creeps: {},
      wave_meta: {
        '1_2_45': {
          lane: 1,
          team: 2,
          spawn_sec: 45,
          last_death_sec: 65,
          last_death_x: 1500,
          last_death_y: 2300,
        },
      },
    };

    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={60}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    const svgElements = document.querySelectorAll('svg');
    expect(svgElements.length).toBe(0);
  });

  it('replaces old pin with newer wave pin after newer wave dies', async () => {
    const data: LaneCreepData = {
      creeps: {},
      wave_meta: {
        '1_2_45': {
          lane: 1,
          team: 2,
          spawn_sec: 45,
          last_death_sec: 65,
          last_death_x: 1500,
          last_death_y: 2300,
        },
        '1_2_90': {
          lane: 1,
          team: 2,
          spawn_sec: 90,
          last_death_sec: 110,
          last_death_x: 1600,
          last_death_y: 2400,
        },
      },
    };

    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={120}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    // Only one pin should show — the newer wave supersedes the older one
    const svgElements = document.querySelectorAll('svg');
    expect(svgElements.length).toBe(1);

    // It should be at the newer wave's position (1600, 2400)
    const indicator = document.querySelector('div[style*="left: 1600px"]');
    expect(indicator).not.toBeNull();
  });

  it('uses team color for live creep dots', async () => {
    const data: LaneCreepData = {
      creeps: {
        '10': [
          { x: 0, y: 0, lane: 1, team: 3, wave_id: '1_3_30', nearby_players: [] },
        ],
      },
      wave_meta: {},
    };

    render(
      <CreepWaveLayer
        laneCreepData={data}
        currentSec={0}
        worldToMinimapPixels={makeWorldToMinimap()}
      />
    );

    const circle = document.querySelector('circle');
    // Sapphire blue (#0EA5E9) for team 3
    expect(circle?.getAttribute('fill')).toBe('#0EA5E9');
  });
});
