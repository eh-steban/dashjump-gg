export interface BossSnapshot {
  entity_index: number;
  custom_id: number; // Entity type ID (21, 25, 26, 27, 28)
  boss_name_hash: string;
  team: number;
  lane: number;
  x: number;
  y: number;
  z: number;
  spawn_time_s: number;
  max_health: number;
  life_state_on_create: number;
  death_time_s: number | null;
  life_state_on_delete: number | null;
}

export interface ScaledBossSnapshot extends BossSnapshot {
  left: number;
  top: number;
}

// Per-second health timeline: object mapping custom_id (as string) -> current_health
type BossHealthWindow = Record<string, number>;

type BossHealthTimeline = BossHealthWindow[];

export interface BossData {
  snapshots: BossSnapshot[];
  health_timeline: BossHealthTimeline;
}

// Map boss_name_hash to human-readable boss type names.
// Hash values are u64 fxhash::hash_bytes of the entity class name, transported
// as decimal strings because JavaScript `number` cannot losslessly hold
// integers above 2^53. Source of truth:
// private/specs/contracts/parser-api.md Boss Type Identification table.
const BOSS_NAME_HASH_MAP: Record<string, string> = {
  '12946736302082733589': 'Guardian',      // CNPC_TrooperBoss, custom_id=21
  '1942975293714691302':  'Walker',        // CNPC_Boss_Tier2, custom_id=28
  '793562361056549792':   'Base Guardian', // CNPC_BarrackBoss, custom_id=26
  '8292725763874089450':  'Shrine',        // CCitadel_Destroyable_Building, custom_id=27
  '7814756300278693755':  'Patron',        // CNPC_Boss_Tier3, custom_id=29
};

export function getBossDisplayName(boss: BossSnapshot): string {
  const typeName =
    BOSS_NAME_HASH_MAP[boss.boss_name_hash] || `Boss #${boss.boss_name_hash}`;
  const laneStr = boss.lane > 0 ? ` - Lane ${boss.lane}` : '';
  let bossName = `${typeName}${laneStr}`;
  if (typeName == 'Base Guardian' || typeName == 'Shrine') {
    bossName = bossName + ` (${boss.entity_index})`;
  }
  return bossName;
}
