import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-react';
import { page } from 'vitest/browser';
import PlayerCards from '../../../src/components/matchAnalysis/PlayerCards';
import { SinnerSnapshot } from '../../../src/domain/sinner';
import { PlayerData, PlayerMatchData } from '../../../src/domain/player';
import { LanePressureData } from '../../../src/domain/lanePressure';
import { ParsedMatchData } from '../../../src/domain/matchAnalysis';

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

function makePlayer(lobby_player_slot: number): PlayerData {
  return {
    custom_id: String(lobby_player_slot),
    entity_id: String(100 + lobby_player_slot),
    hero_id: 1,
    lane: 1,
    lobby_player_slot,
    name: `Player${lobby_player_slot}`,
    steam_id_32: 0,
    team: 2,
    hero: { id: 1, name: 'Kelvin', images: {} },
  };
}

function makePerPlayerData(customId: number): PlayerMatchData {
  return {
    positions: [{ customId: String(customId), x: 0, y: 0, z: 0, is_npc: false }],
    damage: [{}],
  };
}

const EMPTY_LANE_PRESSURE: LanePressureData = { pressure: {} };

const EMPTY_MATCH_DATA: ParsedMatchData = {
  match_duration_s: 0,
  match_start_time_s: 0,
  players_data: [],
  per_player_data: {},
  bosses: { snapshots: [], health_timeline: [] },
  lane_creep_data: { creeps: {}, wave_meta: {} },
  lane_pressure: EMPTY_LANE_PRESSURE,
  sinners: [],
};

function makeSinnerSnapshot(
  retaliation_damage: Record<string, number>,
  overrides: Partial<SinnerSnapshot> = {}
): SinnerSnapshot {
  return {
    entity_index: 1,
    x: 0,
    y: 0,
    z: 0,
    spawn_time_s: 0,
    max_health: 1000,
    death_time_s: null,
    time_alive_s: null,
    killer_player_slot: null,
    retaliation_damage,
    ...overrides,
  };
}

const defaultProps = (
  players: PlayerData[],
  sinners: SinnerSnapshot[]
) => ({
  players,
  perPlayerData: Object.fromEntries(
    players.map((p) => [Number(p.custom_id), makePerPlayerData(Number(p.custom_id))])
  ),
  currentTick: 0,
  matchData: EMPTY_MATCH_DATA,
  lanePressure: EMPTY_LANE_PRESSURE,
  normalizePosition: (_x: number, _y: number) => ({ normX: 0, normY: 0 }),
  sinners,
});

// --------------------------------------------------------------------------
// Tests: Sinner damage taken display
// --------------------------------------------------------------------------

describe('PlayerCards -- sinner damage taken', () => {
  it('shows cumulative damage for a player slot present in retaliation_damage', async () => {
    const player = makePlayer(3);
    const sinners = [
      makeSinnerSnapshot({ '3': 160 }),
      makeSinnerSnapshot({ '3': 320 }),
    ];

    render(<PlayerCards {...defaultProps([player], sinners)} />);

    // Total = 160 + 320 = 480
    await expect.element(page.getByText('480')).toBeInTheDocument();
  });

  it('renders a muted 0 when the player slot is absent from all retaliation_damage maps', async () => {
    const player = makePlayer(5);
    const sinners = [
      makeSinnerSnapshot({ '2': 160 }),
      makeSinnerSnapshot({ '7': 80 }),
    ];

    render(<PlayerCards {...defaultProps([player], sinners)} />);

    // Find all text-gray-400 spans and verify one contains exactly '0'
    const graySpans = document.querySelectorAll('span.text-gray-400');
    const zeroSpan = Array.from(graySpans).find((el) => el.textContent === '0');
    expect(zeroSpan).not.toBeUndefined();
  });

  it('renders 0 when sinners array is empty', async () => {
    const player = makePlayer(1);

    render(<PlayerCards {...defaultProps([player], [])} />);

    const graySpans = document.querySelectorAll('span.text-gray-400');
    const zeroSpan = Array.from(graySpans).find((el) => el.textContent === '0');
    expect(zeroSpan).not.toBeUndefined();
  });

  it('sums multiple sinners that each have damage for the same slot', async () => {
    const player = makePlayer(0);
    const sinners = [
      makeSinnerSnapshot({ '0': 100 }),
      makeSinnerSnapshot({ '0': 200 }),
      makeSinnerSnapshot({ '0': 40 }),
    ];

    render(<PlayerCards {...defaultProps([player], sinners)} />);

    // Total = 100 + 200 + 40 = 340
    await expect.element(page.getByText('340')).toBeInTheDocument();
  });

  it('uses String(lobby_player_slot) as the lookup key', async () => {
    // Verify a numeric slot 7 maps to key '7' and not some other serialization
    const player = makePlayer(7);
    const sinners = [makeSinnerSnapshot({ '7': 99 })];

    render(<PlayerCards {...defaultProps([player], sinners)} />);

    await expect.element(page.getByText('99')).toBeInTheDocument();
  });

  it('shows the Sinner damage taken label on each player card', async () => {
    const player = makePlayer(2);

    render(<PlayerCards {...defaultProps([player], [])} />);

    await expect.element(page.getByText('Sinner damage taken:')).toBeInTheDocument();
  });

  it('does not show damage > 0 as muted -- renders as plain number', async () => {
    const player = makePlayer(4);
    const sinners = [makeSinnerSnapshot({ '4': 500 })];

    render(<PlayerCards {...defaultProps([player], sinners)} />);

    // The number 500 should appear without the gray muted class
    const muted = document.querySelector('.text-gray-400');
    // muted element should not contain 500
    expect(muted?.textContent).not.toBe('500');
    await expect.element(page.getByText('500')).toBeInTheDocument();
  });
});
