//! Entity type hash constants for Deadlock replay parsing

use haste::entities::fkey_from_path;
use haste::fxhash;

// =============================================================================
// Entity Type Hashes
// =============================================================================

/// World entity
pub const CWORLD_ENTITY: u64 = fxhash::hash_bytes(b"CWorld");

/// Game rules proxy
pub const DEADLOCK_GAMERULES_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");

// Player entities
pub const CCITADELPLAYERCONTROLLER_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelPlayerController");
pub const CCITADELPLAYERPAWN_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelPlayerPawn");

// Summonable entities
pub const CNECRO_HAUNTINGSKULL_ENTITY: u64 = fxhash::hash_bytes(b"CNecro_HauntingSkullEntity");
pub const CNPC_NECROSKELE_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_NecroSkele");
pub const CCITADEL_GRAVESTONE_BLOCKER_ENTITY: u64 =
    fxhash::hash_bytes(b"CCitadel_GraveStone_Blocker");

// Lane creeps
pub const CNPC_TROOPER_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_Trooper");

// Drop entities
/// Soul orb spawned on unit death. Players shoot it to claim souls; enemies can deny it.
pub const CITEMXP_ENTITY: u64 = fxhash::hash_bytes(b"CItemXP");

// Objective entities

/// Lane objective -- priority 1 / first target (Guardian). The first structure creeps push
/// toward in each lane. Destroying it opens the path to the Walker.
pub const CNPC_TROOPERBOSS_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_TrooperBoss");
/// Lane objective -- priority 2 (Walker). Second priority target in lane after the Guardian.
pub const CNPC_BOSS_TIER2_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_Boss_Tier2");
/// Lane objective -- priority 3 (Base Guardian). Third priority target after the Walker.
pub const CNPC_BARRACKBOSS_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_BarrackBoss");
/// Lane objective -- priority 4 (Shrine). Destroyable building on each lane route.
/// Must be destroyed before reaching the Base Guardian.
pub const CCITADEL_DESTROYABLE_BUILDING_ENTITY: u64 =
    fxhash::hash_bytes(b"CCitadel_Destroyable_Building");
/// Lane objective -- priority 5 / final (Patron). The main base objective; destroying it ends
/// the match. Last priority in the standard lane-push sequence.
pub const CNPC_BOSS_TIER3_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_Boss_Tier3");
pub const CNPC_MIDBOSS_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_MidBoss");

// Neutral entities
pub const CNPC_TROOPERNEUTRAL_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_TrooperNeutral");
pub const CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY: u64 =
    fxhash::hash_bytes(b"CNPC_Neutral_SinnersSacrifice");

// Defense entities
pub const CNPC_BASE_DEFENSE_SENTRY_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_BaseDefenseSentry");
pub const CNPC_SHIELDEDSENTRY_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_ShieldedSentry");

// Projectile entities (appear as damage attackers, not tracked for position)
pub const CPROJECTILE_PRIEST_SLIDETRAP_ENTITY: u64 =
    fxhash::hash_bytes(b"CProjectile_Priest_SlideTrap_Projectile");

// =============================================================================
// Field Keys
// =============================================================================

// Player controller fields
pub const OWNER_ENTITY_KEY: u64 = fkey_from_path(&["m_hOwnerEntity"]);
pub const PLAYER_NAME_KEY: u64 = fkey_from_path(&["m_iszPlayerName"]);
pub const STEAM_ID_KEY: u64 = fkey_from_path(&["m_steamID"]);
pub const CONTROLLER_HANDLE_KEY: u64 = fkey_from_path(&["m_hPawn"]);
pub const TEAM_KEY: u64 = fkey_from_path(&["m_iTeamNum"]);
pub const ORIGINAL_LANE_ASSIGNMENT_KEY: u64 = fkey_from_path(&["m_nOriginalLaneAssignment"]);
pub const ASSIGNED_LANE_KEY: u64 = fkey_from_path(&["m_nAssignedLane"]);
pub const LANE_SWAP_LOCKED_KEY: u64 = fkey_from_path(&["m_bLaneSwapLocked"]);
pub const HERO_ID_KEY: u64 = fkey_from_path(&["m_PlayerDataGlobal", "m_nHeroID"]);
pub const LOBBY_PLAYER_SLOT_KEY: u64 = fkey_from_path(&["m_unLobbyPlayerSlot"]);

// NPC fields
pub const NPC_LANE_KEY: u64 = fkey_from_path(&["m_iLane"]);
pub const NPC_STATE_KEY: u64 = fkey_from_path(&["m_NPCState"]);

// NPC state values for m_NPCState (underlying type uint32; read as u32 then cast to i32).
// See entity-fields-reference.md for full enum documentation and Deadlock-specific extensions.
/// Invalid / uninitialized sentinel (-1 as i32; stored as u32::MAX in the wire format).
pub const NPC_STATE_INVALID: i32 = -1;
/// Initializing; NPC exists but AI has not started. Treat as not yet active.
pub const NPC_STATE_INIT: i32 = 0;
/// Creep is idle -- active in lane, no threat detected.
pub const NPC_STATE_IDLE: i32 = 1;
/// Creep has detected a threat but not yet engaged.
pub const NPC_STATE_ALERT: i32 = 2;
/// Creep is in combat -- active in lane.
pub const NPC_STATE_COMBAT: i32 = 3;
/// NPC dead (standard SDK state). Reliable death signal for bosses.
/// CNPC_Trooper uses NPC_STATE_DEAD_CITADEL (12) instead -- this value is not observed on troopers.
pub const NPC_STATE_DEAD: i32 = 5;
/// Creep is pre-spawn or recycling back to base -- not yet active in lane.
/// Also used mid-lane for stunned/frozen creeps; not equivalent to "at base".
pub const NPC_STATE_INERT: i32 = 6;
/// Deadlock-specific: CNPC_Trooper death animation playing; co-occurs with LIFE_DYING (life_state=1).
pub const NPC_STATE_DYING_CITADEL: i32 = 10;
/// Deadlock-specific: CNPC_Trooper dead / waiting at base for next wave; co-occurs with LIFE_DEAD
/// (life_state=2). All pre-spawned CNPC_Trooper entities start in this state at match load.
pub const NPC_STATE_DEAD_CITADEL: i32 = 12;

// Life state values for m_lifeState (u8)
pub const LIFE_STATE_KEY: u64 = fkey_from_path(&["m_lifeState"]);
pub const LIFE_ALIVE: u8 = 0;
pub const LIFE_DYING: u8 = 1;
pub const LIFE_DEAD: u8 = 2;

// Movement type field -- CBaseEntity.m_MoveType (MoveType_t, underlying uint8)
pub const MOVE_TYPE_KEY: u64 = fkey_from_path(&["m_MoveType"]);
pub const MOVETYPE_NONE: u8 = 0;
pub const MOVETYPE_STEP: u8 = 9;

// Health field -- CBaseEntity.m_iHealth (int32)
pub const HEALTH_KEY: u64 = fkey_from_path(&["m_iHealth"]);
/// Health sentinel for CNPC_Trooper cage entities (zipline-carrier sprites, not fighting units).
/// Cage entities have m_iHealth == 1; real lane creeps have m_iHealth > 1.
pub const CAGE_ENTITY_HEALTH: i32 = 1;

// Entity creation time -- CBaseEntity.m_flCreateTime (GameTime_t, read as f32)
// Set once when the entity slot is allocated. NOT updated on entity recycling.
// Subtract m_flGameStartTime to get match-relative seconds.
pub const CREATE_TIME_KEY: u64 = fkey_from_path(&["m_flCreateTime"]);

// Spawn flags -- CBaseEntity.m_spawnflags (uint32, bitmask)
pub const SPAWN_FLAGS_KEY: u64 = fkey_from_path(&["m_spawnflags"]);
