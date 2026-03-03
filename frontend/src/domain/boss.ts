export interface BossSnapshot {
  entity_index: number;
  custom_id: number; // Entity type ID (21, 25, 26, 27, 28)
  boss_name_hash: number;
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

// Map boss_name_hash to human-readable boss type names
// Hash values are fxhash of entity class names in the game
// These hash values come from the parser.
// TODO: These hard coded hash names changed and I'm unsure if
// these are different between games or if the big update a
// few weeks ago (date today: 3/3/26) broke things. Will
// likely need to think of a more resilient option especially
// if this changes every game. This change not only broke the
// naming in the UI, but the sankey diagrams too (because there's
// duplicate hash values and the diagrams don't like dupes).
const BOSS_NAME_HASH_MAP: Record<string, string> = {
  '10648152268083397000': 'Guardian', // CNPC_TrooperBoss, custom_id=21
  '14993025469191344000': 'Walker', // CNPC_Boss_Tier2, custom_id=28
  '7661004720742107000': 'Base Guardian', // CNPC_BarrackBoss, custom_id=26
  '3692976131341581000': 'Shrine', // CCitadel_Destroyable_Building, custom_id=27
  '9121244462627342000': 'Patron', // CNPC_Boss_Tier3, custom_id=29
};

export function getBossDisplayName(boss: BossSnapshot): string {
  const hashKey = String(boss.boss_name_hash);
  const typeName =
    BOSS_NAME_HASH_MAP[hashKey] || `Boss #${boss.boss_name_hash}`;
  const laneStr = boss.lane > 0 ? ` - Lane ${boss.lane}` : '';
  let bossName = `${typeName}${laneStr}`;
  if (typeName == 'Base Guardian' || typeName == 'Shrine') {
    bossName = bossName + ` (${boss.entity_index})`;
  }
  return bossName;
}
