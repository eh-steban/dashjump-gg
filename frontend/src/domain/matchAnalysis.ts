import { MatchMetadata } from "./matchMetadata";
import { ParsedPlayer, PlayerMatchData } from "./player";
import { BossData } from "./boss";
import { LaneCreepData } from "./creep";
import { LanePressureData } from "./lanePressure";
import { SinnerSnapshot } from "./sinner";

// Parsed match data (aggregated by player, per backend ParsedMatchData)
export interface ParsedMatchData {
  match_duration_s: number;
  match_start_time_s: number;
  players_data: ParsedPlayer[];
  per_player_data: Record<string, PlayerMatchData>; // key = player_id
  bosses: BossData;
  lane_creep_data: LaneCreepData;
  lane_pressure: LanePressureData;
  sinners: SinnerSnapshot[];
}

// Full match analysis response (backend MatchAnalysis)
// Note: npc keys are strings (backend returns dict[str, NPC])
export interface MatchAnalysisResponse {
  match_metadata: MatchMetadata;
  parsed_match_data: ParsedMatchData;
}

// *****NOTE******
// These values can be found in the parser under...
// m_pGameRules.m_vMinimapMins:Vector = [-10752.0, -10752.0, 0.0]
// m_pGameRules.m_vMinimapMaxs:Vector = [10752.0, 10752.0, 0.0]
// After checking 2 games, the values seem to be constant.
// The parser docs show a different value, so I'm wondering if they
// ever change. For now, we'll hardcode them.
const MINIMAP_MIN = -10752;
const MINIMAP_MAX = 10752;

export const WORLD_BOUNDS = Object.freeze({
  xMin: MINIMAP_MIN,
  xMax: MINIMAP_MAX,
  yMin: MINIMAP_MIN,
  yMax: MINIMAP_MAX,
} as const);

export const defaultMatchAnalysis: MatchAnalysisResponse = {
  match_metadata: {
    match_info: {
      duration_s: 0,
      match_outcome: 0,
      winning_team: 0,
      players: [],
      start_time: 0,
      match_id: 0,
      legacy_objectives_mask: null,
      game_mode: 0,
      match_mode: 0,
      objectives: [],
      damage_matrix: {
        sample_time_s: [],
        source_details: {
          stat_type: [],
          source_name: [],
        },
        damage_dealers: [],
      },
      match_pauses: [],
      customer_user_stats: undefined,
      watched_death_replays: [],
      objectives_mark_team0: undefined,
      objectives_mark_team1: undefined,
      mid_boss: [],
      is_high_skill_range_parties: false,
      low_pri_pool: false,
      new_player_pool: false,
      average_badge_team0: 0,
      average_badge_team1: 0,
      game_mode_version: 0,
    },
  },
  parsed_match_data: {
    match_duration_s: 0,
    match_start_time_s: 0,
    players_data: [],
    per_player_data: {},
    bosses: {
      snapshots: [],
      health_timeline: [],
    },
    lane_creep_data: {
      creeps: {},
      wave_meta: {},
    },
    lane_pressure: {
      pressure: {},
    },
    sinners: [],
  },
};
