export interface DRTypeAggregateBySec {
  agg_damage: number;
  agg_pre_damage: number;
  type: number;
  citadel_type: number;
  entindex_inflictor: number;
  entindex_ability: number;
  agg_damage_absorbed: number;
  victim_health_max: number;
  victim_health_new: number;
  flags: number;
  ability_id: number;
  attacker_class: number;
  victim_class: number;
  victim_shield_max: number;
  agg_victim_shield_new: number;
  agg_hits: number;
  agg_health_lost: number;
}

export interface DamageRecord {
  damage?: number;
  pre_damage?: number;
  type?: number;
  citadel_type?: number;
  entindex_inflictor?: number;
  entindex_ability?: number;
  damage_absorbed?: number;
  victim_health_max?: number;
  victim_health_new?: number;
  flags?: number;
  ability_id?: number;
  attacker_class?: number;
  victim_class?: number;
  victim_shield_max?: number;
  victim_shield_new?: number;
  hits?: number;
  health_lost?: number;
}

// victim_id -> DamageRecord[]
export type ParsedVictimDamage = Record<string, DamageRecord[]>;
/**
 * Damage timeline:
 * Index = tick (second). Each element is either:
 *  - ParsedVictimDamage (victim -> damage records)
 *  - null (no damage that tick)
 */
export type Damage = ParsedVictimDamage[];

// Comes from parser
export interface PlayerPosition {
  customId: string;
  x: number;
  y: number;
  z: number;
  is_npc: boolean;
}

// Scaled coordinates for minimap rendering
export interface ScaledPlayerCoord extends PlayerPosition {
  left: number;
  top: number;
}

// All positions for one tick
export type PositionWindow = PlayerPosition[];

// Comes from parser
export interface PlayerData {
  custom_id: string;
  entity_id: string;
  hero_id: number; // Raw hero ID from backend (used for initial hero lookup)
  lane: number;
  lobby_player_slot: number;
  name: string;
  steam_id_32: number;
  team: number;
  hero: Hero; // Enriched hero data (added after lookup, may be fallback if hero_id not found)
}

// Per-player aggregated data (damage + positions windows)
export interface PlayerMatchData {
  positions: PositionWindow;
  damage: Damage;
}

// Temporary ParsedPlayer type (extend if backend adds extra fields)
export interface ParsedPlayer extends PlayerData {
  // Add any additional parsed-only fields here if needed
}

export interface Hero {
  id: number;
  name: string;
  images: { [key: string]: string };
  minimapImage?: string;
  heroCardWebp?: string;
}

// Comes from parser
export interface NPC {
  entity_id: number;
  name: string;
}
