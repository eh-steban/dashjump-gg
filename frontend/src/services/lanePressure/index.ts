import { LanePressureData } from '../../domain/lanePressure';

export interface PlayerLanePressureEntry {
  lane: string;
  team: number;
  pressure: number; // 0–1
}

/**
 * Returns the lane pressure entries attributed to a player at a given tick.
 * Keys follow the format "{lane}_{team}_{waveIdx}" (e.g. "1_2_0").
 * Groups by lane+team, keeping the highest-pressure wave per group.
 */
export function getPlayerLanePressure(
  playerCustomId: number,
  lanePressure: LanePressureData,
  currentTick: number
): PlayerLanePressureEntry[] {
  // lane+team key → highest pressure seen
  const bestByGroup = new Map<string, PlayerLanePressureEntry>();

  for (const [key, snapshots] of Object.entries(lanePressure.pressure)) {
    if (!snapshots || currentTick >= snapshots.length) continue;

    const snapshot = snapshots[currentTick];
    if (!snapshot) continue;
    if (!snapshot.attributed_players.includes(playerCustomId)) continue;

    const parts = key.split('_');
    const lane = parts[0];
    const team = parseInt(parts[1]);
    const groupKey = `${lane}_${team}`;

    const existing = bestByGroup.get(groupKey);
    if (!existing || snapshot.pressure > existing.pressure) {
      bestByGroup.set(groupKey, { lane, team, pressure: snapshot.pressure });
    }
  }

  return Array.from(bestByGroup.values()).sort((a, b) =>
    a.lane.localeCompare(b.lane)
  );
}
