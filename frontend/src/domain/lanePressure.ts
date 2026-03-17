// Lane pressure types for map control tracking

export interface LanePressureSnapshot {
  pressure: number; // 0-1 value (0 = own base, 1 = enemy base)
  team: number; // Team (2 = Amber, 3 = Sapphire)
  wave_id: string;
  creep_count: number;
  attributed_players: number[]; // Player custom_ids within proximity
}

// Key format: wave_id (e.g., "1_2_45" for lane 1, team 2, spawned at sec 45)
// Value: per-second snapshots (null if wave not active that second)
export interface LanePressureData {
  pressure: Record<string, (LanePressureSnapshot | null)[]>;
}
