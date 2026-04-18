export interface DamageTarget {
  id: string;
  name: string;
  type: 'player' | 'boss' | 'lane_creeps' | 'neutral_creeps';
  damage: number;
  percentage?: number;
  team?: number;
  heroImage?: string;
  bossNameHash?: string;
}

export interface PlayerDamageDistribution {
  totalDamage: number;
  targets: DamageTarget[];
}

export interface TeamDamageDistribution {
  totalDamage: number;
  targets: DamageTarget[];
}

export interface PlayerContribution {
  playerId: string;
  playerName: string;
  heroName: string;
  damage: number;
  percentage: number;
  heroImage: string;
}

export interface ObjectiveTarget extends DamageTarget {
  spawnTime: number;
  deathTime: number | null;
}

export interface PlayerObjectiveDamage {
  playerId: string;
  playerName: string;
  objectiveId: string;
  objectiveName: string;
  damage: number;
}

export interface ObjectiveDamageDistribution {
  totalObjectiveDamage: number;
  playerContributions: PlayerContribution[];
  objectiveTargets: ObjectiveTarget[];
  playerObjectiveDamageMatrix: PlayerObjectiveDamage[];
}
