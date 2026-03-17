// Creep lane tracking types for per-creep position data

export interface CreepSnapshot {
  x: number;
  y: number;
  lane: number;
  team: number;
  wave_id: string;
  nearby_players: number[];
}

export interface WaveMeta {
  lane: number;
  team: number;
  spawn_sec: number;
  last_death_sec: number | null;
  last_death_x: number | null;
  last_death_y: number | null;
}

// creeps: string entity_index -> per-second timeline of snapshots (null if not alive that second)
// wave_meta: wave_id -> wave metadata including spawn/death info for pin rendering
export interface LaneCreepData {
  creeps: Record<string, (CreepSnapshot | null)[]>;
  wave_meta: Record<string, WaveMeta>;
}
