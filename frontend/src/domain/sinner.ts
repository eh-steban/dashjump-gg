export interface SinnerSnapshot {
  entity_index: number;
  x: number;
  y: number;
  z: number;
  spawn_time_s: number;
  max_health: number;
  death_time_s: number | null;
  time_alive_s: number | null;
  killer_player_slot: number | null;
  // Backend serializes Rust HashMap<u32, i32> keys as strings
  retaliation_damage: Record<string, number>;
}

export interface ScaledSinnerSnapshot extends SinnerSnapshot {
  left: number;
  top: number;
}
