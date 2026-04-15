//! Main replay parser - coordinates parsing of Deadlock demo files

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity, ehandle_to_index, fkey_from_path};
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::deadlock::{
    CCitadelUserMessageDamage, CCitadelUserMsgBossKilled, CCitadelUserMsgMidBossSpawned,
    CCitadelUserMsgPostMatchDetails, CCitadelUserMsgRejuvStatus, CMsgMatchMetaDataContentsPatched,
    CitadelUserMessageIds,
};
use prost::Message;
use tracing::{debug, error, info, warn};

use crate::domain::{DamageRecord, Player, PlayerPosition};
// Note: CreepSnapshot, LaneCreepData etc. are used via CreepTracker::get_output()
use crate::entities::constants::*;
use crate::entities::constants::MAX_HEALTH_KEY;
use crate::entities::constants::MID_BOSS_CLASS_ID;
use crate::tracking::{BossTracker, CreepTracker, MidBossTracker, SinnerTracker};
use crate::utils::{get_entity_position, get_steam_id32};

/// Thin error wrapper for the Visitor boundary.
/// Internal helpers use anyhow::Result; the From impls let `?` convert automatically.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParseError(#[from] anyhow::Error);

impl From<prost::DecodeError> for ParseError {
    fn from(e: prost::DecodeError) -> Self {
        ParseError(anyhow::Error::from(e))
    }
}

/// Main visitor that collects all match data during parsing
#[derive(Debug)]
struct MyVisitor {
    current_match_second: u32,
    match_start_time_s: Option<u32>,
    damage_window: HashMap<u32, HashMap<u32, Vec<DamageRecord>>>,
    damage: Vec<HashMap<u32, HashMap<u32, Vec<DamageRecord>>>>,
    players: Vec<Player>,
    entity_name_hash_to_player_slot: HashMap<u32, u32>,
    positions_window: Vec<PlayerPosition>,
    positions: Vec<Vec<PlayerPosition>>,
    boss_tracker: BossTracker,
    creep_tracker: CreepTracker,
    mid_boss_tracker: MidBossTracker,
    sinner_tracker: SinnerTracker,
    lane_data_updated: bool,
    /// Serializer hashes for which we've already logged an "owned NPC attacker failed
    /// to resolve to a player slot" error in this parse session. Used to dedupe the
    /// warning so a new owned-NPC class shows up once per replay instead of once per
    /// damage event.
    warned_unresolvable_attacker_classes: HashSet<u64>,
}

impl Default for MyVisitor {
    fn default() -> Self {
        let boss_tracker = BossTracker::new();
        let creep_tracker = CreepTracker::new(boss_tracker.lane_key());
        let mid_boss_tracker = MidBossTracker::new();
        let sinner_tracker = SinnerTracker::new();
        Self {
            current_match_second: 0,
            match_start_time_s: None,
            damage_window: HashMap::new(),
            damage: Vec::new(),
            players: Vec::new(),
            entity_name_hash_to_player_slot: HashMap::new(),
            positions_window: Vec::new(),
            positions: Vec::new(),
            boss_tracker,
            creep_tracker,
            mid_boss_tracker,
            sinner_tracker,
            lane_data_updated: false,
            warned_unresolvable_attacker_classes: HashSet::new(),
        }
    }
}

impl MyVisitor {
    /// Finalize all trackers that require post-parse computation.
    /// Must be called before get_match_data_json().
    pub fn finalize_trackers(&mut self) {
        self.mid_boss_tracker.finalize();
    }

    /// Get final match data as JSON
    pub fn get_match_data_json(&self) -> serde_json::Value {
        let lane_creep_data = self.creep_tracker.get_output();

        serde_json::json!({
            "match_duration_s": self.current_match_second,
            "match_start_time_s": self.match_start_time_s,
            "damage": self.damage,
            "players": self.players,
            "positions": self.positions,
            "bosses": self.boss_tracker.get_output(),
            "lane_creep_data": lane_creep_data,
            "sinners": self.sinner_tracker.get_output(),
            "mid_boss": self.mid_boss_tracker.get_output(),
        })
    }

    /// Check if lane swap is locked and update player lane/color
    fn check_and_update_lane_lock(&mut self, entity: &Entity) -> Result<()> {
        if !entity.serializer_name_heq(CCITADELPLAYERCONTROLLER_ENTITY) {
            return Ok(());
        }

        let is_locked: bool = entity.get_value(&LANE_SWAP_LOCKED_KEY).unwrap_or(false);
        if !is_locked {
            return Ok(());
        }

        // Player is lane swap locked. Update lane and color
        let lobby_slot: u32 = entity.get_value(&LOBBY_PLAYER_SLOT_KEY).unwrap_or(9999);
        for player in &mut self.players {
            if player.lobby_player_slot == lobby_slot {
                // NOTE: Fields have been changed to int8 in the demo. Previously u32. This
                // created a bug where we got none instead of the lane value. If we encounter
                // an issue in the future where values aren't coming through the API from here
                // let's check that we're using the correct type.
                player.lane = entity
                    .get_value::<i8>(&ASSIGNED_LANE_KEY)
                    .filter(|&v| v != 0)
                    .unwrap_or(99);

                break;
            }
        }

        // Check if all players have lane data now
        let all_lanes_set = self.players.len() >= 12 && self.players.iter().all(|p| p.lane != 99);
        if all_lanes_set {
            self.lane_data_updated = true;
        }

        Ok(())
    }

    /// Check if entity should have its position tracked.
    /// Lane creeps (troopers) require a lane assignment to filter out pre-spawned units.
    fn should_track_position(&self, entity: &Entity) -> bool {
        let hash = entity.serializer().serializer_name.hash;

        // Lane creeps require a lane assignment (pre-spawned units have lane 0)
        if hash == CNPC_TROOPER_ENTITY {
            let lane: i32 = entity.get_value(&NPC_LANE_KEY).unwrap_or(0);
            return lane != 0;
        }

        matches!(
            hash,
            CCITADELPLAYERPAWN_ENTITY
                | CNPC_TROOPERBOSS_ENTITY
                | CNPC_TROOPERNEUTRAL_ENTITY
                | CITEMXP_ENTITY
                | CCITADEL_DESTROYABLE_BUILDING_ENTITY
                | CNPC_BARRACKBOSS_ENTITY
                | CNPC_BOSS_TIER2_ENTITY
                | CNPC_BOSS_TIER3_ENTITY
                | CNPC_BASE_DEFENSE_SENTRY_ENTITY
                | CNPC_SHIELDEDSENTRY_ENTITY
        )
    }

    /// Check if entity needs a one-time position snapshot at CREATE.
    /// Used for stationary objectives that do not require per-second tracking.
    fn should_track_snapshot(&self, entity: &Entity) -> bool {
        let hash = entity.serializer().serializer_name.hash;
        matches!(hash, CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY)
    }

    /// Check if entity is an NPC (not a player pawn)
    fn is_npc_entity(&self, entity: &Entity) -> bool {
        entity.serializer().serializer_name.hash != CCITADELPLAYERPAWN_ENTITY
    }

    /// Handle game rules entity to extract match start time
    fn handle_game_rules(&mut self, entity: &Entity, tick: i32) -> Result<()> {
        debug_assert!(entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY));

        let match_start_time_s_f: f32 =
            entity.try_get_value(&fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]))?;

        if match_start_time_s_f < 0.001 {
            return Ok(());
        }

        let rounded: u32 = match_start_time_s_f.round() as u32;

        if self.match_start_time_s.is_none() {
            info!(
                "[parse_replay] m_flGameStartTime first seen: tick={}, raw={:.4}, rounded={}",
                tick, match_start_time_s_f, rounded
            );
        }

        self.match_start_time_s = Some(rounded);

        Ok(())
    }

    /// Get custom ID for an entity (player slot for players, fixed ID for NPCs)
    fn get_custom_id(&mut self, ctx: &Context, entity: &Entity) -> u32 {
        let serializer_entity_name = &entity.serializer().serializer_name;

        if serializer_entity_name.hash == CCITADELPLAYERPAWN_ENTITY {
            return *self
                .entity_name_hash_to_player_slot
                .entry(entity.index() as u32)
                .or_insert_with(|| {
                    let owner_entity_index: u32 =
                        entity.get_value::<u32>(&OWNER_ENTITY_KEY).unwrap();
                    let owner_entity = ctx
                        .entities()
                        .unwrap()
                        .get(&ehandle_to_index(owner_entity_index))
                        .unwrap();
                    let lobby_player_slot = owner_entity
                        .get_value(&LOBBY_PLAYER_SLOT_KEY)
                        .unwrap_or(999999);

                    // String values in haste are now Box<[u8]> since not all demo strings are valid UTF-8
                    let player_name: String = owner_entity
                        .get_value::<Box<[u8]>>(&PLAYER_NAME_KEY)
                        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let steam_id = get_steam_id32(owner_entity).unwrap_or(999999);
                    let hero_id: u32 = owner_entity.get_value(&HERO_ID_KEY).unwrap_or(999999);
                    let team: u32 = owner_entity.get_value(&TEAM_KEY).unwrap_or(999999);

                    self.players.push(Player {
                        entity_id: entity.index().to_string(),
                        custom_id: lobby_player_slot.to_string(),
                        name: player_name,
                        steam_id_32: steam_id,
                        hero_id,
                        lobby_player_slot,
                        team,
                        lane: owner_entity
                            .get_value::<i8>(&ASSIGNED_LANE_KEY)
                            .filter(|&v| v != 0)
                            .unwrap_or(99),
                    });

                    lobby_player_slot
                });
        }

        // NPC entities get fixed IDs
        match serializer_entity_name.hash {
            CNPC_TROOPER_ENTITY => 20,
            CNPC_TROOPERBOSS_ENTITY => 21,
            CNPC_TROOPERNEUTRAL_ENTITY => 22,
            CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY => 23,
            CITEMXP_ENTITY => 25,
            CNPC_BARRACKBOSS_ENTITY => 26,
            CCITADEL_DESTROYABLE_BUILDING_ENTITY => 27,
            CNPC_BOSS_TIER2_ENTITY => 28,
            CNPC_BOSS_TIER3_ENTITY => 29,
            CNPC_BASE_DEFENSE_SENTRY_ENTITY => 30,
            CNPC_SHIELDEDSENTRY_ENTITY => 31,
            CCITADELPLAYERCONTROLLER_ENTITY => 32,
            CNECRO_HAUNTINGSKULL_ENTITY => 33,
            CNPC_NECROSKELE_ENTITY => 34,
            CCITADEL_GRAVESTONE_BLOCKER_ENTITY => 35,
            _ => panic!(
                "Unknown entity - Index: {}, Hash: {}",
                entity.index(),
                serializer_entity_name.hash,
            ),
        }
    }

    /// Get entity ID for damage tracking (entity index for bosses/projectiles, custom_id otherwise)
    fn get_damage_entity_id(&mut self, ctx: &Context, entity: &Entity) -> u32 {
        let hash = entity.serializer().serializer_name.hash;
        if hash == CCITADELPLAYERPAWN_ENTITY {
            return self.get_custom_id(ctx, entity);
        }
        if self.boss_tracker.is_boss_entity(hash) {
            return entity.index() as u32;
        }
        // Mid-boss appears as both attacker and victim in damage events but is not
        // in get_custom_id's NPC match (fixed IDs would collide across spawn cycles).
        // Use entity index for per-instance identity, mirroring the priest slide-trap pattern.
        if hash == CNPC_MIDBOSS_ENTITY {
            return entity.index() as u32;
        }
        // Projectiles appear as damage sources but aren't position-tracked entities.
        // Use entity index to uniquely identify them without a fixed ID.
        if hash == CPROJECTILE_PRIEST_SLIDETRAP_ENTITY {
            return entity.index() as u32;
        }
        self.get_custom_id(ctx, entity)
    }

    /// Record a damage event
    fn push_damage_record(
        &mut self,
        ctx: &Context,
        attacker: &Entity,
        victim: &Entity,
        record: DamageRecord,
    ) -> Result<()> {
        let attacker_id = self.get_damage_entity_id(ctx, attacker);
        let victim_id = self.get_damage_entity_id(ctx, victim);

        let victims_list = self
            .damage_window
            .entry(attacker_id)
            .or_insert(HashMap::new());

        let victim_damage = victims_list.entry(victim_id).or_insert(Vec::new());

        victim_damage.push(record);

        Ok(())
    }

    /// Resolve a damage-event attacker to the owning player's lobby slot.
    ///
    /// Handles two attacker shapes:
    /// - Direct `CCitadelPlayerPawn` -- resolved via controller and lobby slot.
    /// - NPC proxy owned by a player (Lil Helper, etc.) -- chases `m_hOwnerEntity`
    ///   to the owner pawn, then resolves as above.
    ///
    /// Returns `None` for unowned attackers (troopers, neutral NPCs) where no
    /// player slot can be attributed. In that case the caller should silently
    /// skip any Dealt event -- kill attribution still fires via `record_damage`.
    ///
    /// If the attacker advertises an `m_hOwnerEntity` but the resulting chain
    /// fails to yield a player slot, the attacker class is an owned NPC we do
    /// not know how to resolve -- most likely a new owned-unit class shipped by
    /// Valve. Emits an `error!` once per serializer hash per parse session so
    /// we can extend `resolve_attacker_to_player_slot` and re-parse affected
    /// replays to recover the missing Dealt events.
    fn resolve_attacker_to_player_slot(
        &mut self,
        attacker: &Entity,
        ctx: &Context,
    ) -> Option<u32> {
        if attacker.serializer_name_heq(CCITADELPLAYERPAWN_ENTITY) {
            return resolve_pawn_to_slot(attacker.index(), ctx);
        }
        let owner_handle: u32 = attacker.get_value(&OWNER_ENTITY_KEY)?;
        let owner_idx = ehandle_to_index(owner_handle);
        let slot = resolve_pawn_to_slot(owner_idx, ctx);
        if slot.is_none() {
            let class_hash = attacker.serializer().serializer_name.hash;
            if self
                .warned_unresolvable_attacker_classes
                .insert(class_hash)
            {
                error!(
                    "[parse_replay] Owned NPC attacker could not be resolved to a player slot; \
                     Sinner Dealt event skipped for this class. \
                     class_hash=0x{:016x} attacker_entity_index={} owner_handle={} \
                     owner_entity_index={} match_second={}. \
                     If this represents a player-owned unit (new minion or ability summon), \
                     extend MyVisitor::resolve_attacker_to_player_slot and re-parse affected \
                     replays to recover the missing Dealt events.",
                    class_hash,
                    attacker.index(),
                    owner_handle,
                    owner_idx,
                    self.current_match_second,
                );
            }
        }
        slot
    }
}

/// Resolve a player pawn entity index to a lobby player slot.
/// Walks the pawn -> controller -> m_unLobbyPlayerSlot chain.
/// Returns `None` if any step of the chain is absent.
fn resolve_pawn_to_slot(pawn_index: i32, ctx: &Context) -> Option<u32> {
    let entities = ctx.entities()?;
    let pawn = entities.get(&pawn_index)?;
    let owner_handle: u32 = pawn.get_value(&OWNER_ENTITY_KEY).unwrap_or(0);
    let controller_index = ehandle_to_index(owner_handle);
    let controller = entities.get(&controller_index)?;
    let slot: u32 = controller.get_value(&LOBBY_PLAYER_SLOT_KEY)?;
    Some(slot)
}


impl Visitor for &mut MyVisitor {
    type Error = ParseError;

    async fn on_tick_end(
        &mut self,
        ctx: &Context,
    ) -> Result<(), ParseError> {
        let next_window = (((1 + ctx.tick()) as f32) * ctx.tick_interval()).round() as u32;
        let this_window = ((ctx.tick() as f32) * ctx.tick_interval()).round() as u32;

        let match_started =
            self.match_start_time_s.is_some() && (this_window >= self.match_start_time_s.unwrap());

        if !match_started {
            return Ok(());
        }

        if next_window != this_window {

            // Build per-second boss health and creep wave timelines using match-relative time
            // (0-indexed from match second 0, matching the player positions array)
            let match_window = this_window - self.match_start_time_s.unwrap();

            // Pad skipped seconds. Tick-to-second rounding can cause match_window to jump
            // (e.g. 100 → 102, skipping 101). Positions/damage/boss health must have an
            // entry for every second so that array index == match second.
            let expected_len = match_window as usize;
            while self.positions.len() < expected_len {
                // Repeat the last known positions frame (or empty if none yet)
                let pad = self.positions.last().cloned().unwrap_or_default();
                self.positions.push(pad);
                // Empty damage for the skipped second
                self.damage.push(HashMap::new());
                // Carry forward boss health for the skipped second
                self.boss_tracker.build_health_window(self.positions.len() as u32 - 1);
            }

            self.boss_tracker.build_health_window(match_window);

            // Collect player positions this tick for nearby-player computation in creep snapshots.
            // We scan all player pawns and convert custom_id to i32 for the tracker API.
            let mut player_positions_for_creeps: Vec<(i32, f32, f32)> = Vec::new();

            // Collect positions for all tracked entities
            for (_index, entity) in ctx.entities().unwrap().iter() {
                if !self.should_track_position(entity) {
                    continue;
                }
                let Some(position) = get_entity_position(entity) else {
                    error!(
                        "[parse_replay] tracked entity has no position (bug): Hash={}, Index={}",
                        entity.serializer().serializer_name.hash,
                        entity.index(),
                    );
                    continue;
                };
                let custom_id = self.get_custom_id(ctx, entity);
                let is_npc = self.is_npc_entity(entity);

                // Collect player pawn positions for creep nearby-player computation
                if !is_npc {
                    player_positions_for_creeps.push((custom_id as i32, position[0], position[1]));
                }

                self.positions_window.push(PlayerPosition {
                    custom_id: custom_id.to_string(),
                    x: position[0],
                    y: position[1],
                    z: position[2],
                    is_npc,
                });
            }

            // Build creep snapshots with current player positions for proximity tracking
            self.creep_tracker
                .build_creep_snapshot(match_window, &player_positions_for_creeps);

            self.damage.push(std::mem::take(&mut self.damage_window));
            self.positions
                .push(std::mem::take(&mut self.positions_window));
            self.current_match_second = match_window;
        }

        Ok(())
    }

    async fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), ParseError> {
        // Handle game rules for match start time
        if entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY) {
            self.handle_game_rules(entity, ctx.tick())?;
        }

        // Track boss and creep lifecycle
        let hash = entity.serializer().serializer_name.hash;

        // Only track creeps after match has started to avoid pre-match entities
        let match_started = self.match_start_time_s.is_some();

        // Route mid-boss entity events to the mid-boss tracker.
        // Mid-boss is no longer position-tracked -- its lifecycle is managed exclusively here.
        if hash == CNPC_MIDBOSS_ENTITY {
            match delta_header {
                DeltaHeader::CREATE => {
                    self.mid_boss_tracker.observe_entity(entity.index(), entity, true);
                }
                DeltaHeader::UPDATE => {
                    self.mid_boss_tracker.observe_entity(entity.index(), entity, false);
                }
                _ => {}
            }
        }

        match delta_header {
            DeltaHeader::CREATE => {
                if self.boss_tracker.is_boss_entity(hash) {
                    let custom_id = self.get_custom_id(ctx, entity);
                    let match_time_s = self.current_match_second;
                    self.boss_tracker.handle_boss_create(
                        entity,
                        custom_id,
                        hash,
                        match_time_s,
                    );
                }
                // Only track creeps after match starts and once lane is assigned.
                // Lane 0 means the entity exists but hasn't been assigned to a lane yet
                // (pre-spawn state). Registering it would produce a frozen ghost creep
                // because all subsequent UPDATE events also arrive with lane=0 and are
                // filtered out before updating its position.
                if match_started && CreepTracker::is_creep_entity(hash) {
                    let team: u32 = entity.get_value(&TEAM_KEY).unwrap_or(0);
                    let lane: i32 = entity.get_value(&NPC_LANE_KEY).unwrap_or(0);
                    let npc_state: i32 = entity
                        .get_value::<u32>(&NPC_STATE_KEY)
                        .map(|v| v as i32)
                        .unwrap_or(NPC_STATE_INVALID);
                    let life_state: u8 = entity
                        .get_value::<u8>(&LIFE_STATE_KEY)
                        .unwrap_or(LIFE_ALIVE);
                    let health: i32 = entity.get_value(&HEALTH_KEY).unwrap_or(0);
                    if lane != 0 {
                        let Some(position) = get_entity_position(entity) else {
                            error!(
                                "[parse_replay] creep entity has no position on CREATE (bug): Index={}",
                                entity.index()
                            );
                            return Ok(());
                        };
                        let match_time_s = self.current_match_second;
                        self.creep_tracker.handle_creep_create(
                            entity.index(),
                            lane,
                            team,
                            position[0],
                            position[1],
                            match_time_s,
                            npc_state,
                            life_state,
                            health,
                        );
                    }
                }
                if match_started && self.should_track_snapshot(entity) {
                    let Some(position) = get_entity_position(entity) else {
                        error!(
                            "[parse_replay] sinner entity has no position on CREATE (bug): Index={}",
                            entity.index()
                        );
                        return Ok(());
                    };
                    let max_health: i32 = entity
                        .get_value::<i32>(&MAX_HEALTH_KEY)
                        .unwrap_or(500);
                    let match_time_s = self.current_match_second;
                    self.sinner_tracker.handle_sinner_create(
                        entity.index(),
                        position[0],
                        position[1],
                        position[2],
                        max_health,
                        match_time_s,
                    );
                }
            }
            DeltaHeader::DELETE => {
                let match_time_s = self.current_match_second;
                self.boss_tracker
                    .handle_boss_delete(entity, match_time_s);
                if CreepTracker::is_creep_entity(hash) {
                    self.creep_tracker
                        .handle_creep_delete(entity.index(), match_time_s);
                }
            }
            DeltaHeader::UPDATE => {
                // Check for lane lock updates
                if !self.lane_data_updated {
                    self.check_and_update_lane_lock(entity)?;
                }
                // Sample boss state (max_health + current health) on every UPDATE.
                // Captures non-damage HP changes: Walker/Shrine sibling-scaling
                // heals, Patron phase 1->2 reset, Patron time-based scaling,
                // and out-of-combat regen.
                if self.boss_tracker.is_boss_entity(hash) {
                    let match_time_s = self.current_match_second;
                    self.boss_tracker.handle_boss_update(entity, match_time_s);
                }
                // Update creep positions (only if match started).
                // Lane=0 filtering is handled inside handle_creep_update: already-registered
                // creeps always receive position updates (their lane may transiently read 0
                // during the zipline-to-walking transition), while unregistered entities with
                // lane=0 are skipped to prevent ghost creep registration.
                if match_started && CreepTracker::is_creep_entity(hash) {
                    let Some(position) = get_entity_position(entity) else {
                        error!(
                            "[parse_replay] creep entity has no position on UPDATE (bug): Index={}",
                            entity.index()
                        );
                        return Ok(());
                    };
                    let team: u32 = entity.get_value(&TEAM_KEY).unwrap_or(0);
                    let lane: i32 = entity.get_value(&NPC_LANE_KEY).unwrap_or(0);
                    let npc_state: i32 = entity
                        .get_value::<u32>(&NPC_STATE_KEY)
                        .map(|v| v as i32)
                        .unwrap_or(NPC_STATE_INVALID);
                    let life_state: u8 = entity
                        .get_value::<u8>(&LIFE_STATE_KEY)
                        .unwrap_or(LIFE_ALIVE);
                    let health: i32 = entity.get_value(&HEALTH_KEY).unwrap_or(0);
                    let match_time_s = self.current_match_second;
                    self.creep_tracker.handle_creep_update(
                        entity.index(),
                        lane,
                        team,
                        position[0],
                        position[1],
                        match_time_s,
                        npc_state,
                        life_state,
                        health,
                    );
                }
                if match_started && hash == CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY {
                    if let Some(health) = entity.get_value::<i32>(&HEALTH_KEY) {
                        let match_time_s = self.current_match_second;
                        if let Some(attacker_entity_index) = self.sinner_tracker
                            .handle_sinner_update(entity.index(), health, match_time_s)
                        {
                            // Death detected -- resolve attacker entity index to lobby player slot.
                            // A6.5 defensive fallback: if the attacker is not a player pawn,
                            // attempt m_hOwnerEntity resolution (handles potential Lil Helper case).
                            let attacker_index = attacker_entity_index;
                            let attacker_entity = ctx
                                .entities()
                                .and_then(|e| e.get(&attacker_index));
                            let attacker_hash = attacker_entity
                                .map(|e| e.serializer().serializer_name.hash);

                            let killer_slot = match attacker_hash {
                                Some(h) if h == CCITADELPLAYERPAWN_ENTITY => {
                                    // Normal path: attacker is a player pawn
                                    resolve_pawn_to_slot(attacker_index, ctx)
                                }
                                Some(_) => {
                                    // Defensive path: attacker is an NPC (e.g., possible Lil Helper)
                                    // Try m_hOwnerEntity to find the owning player pawn
                                    let owner_slot = attacker_entity
                                        .and_then(|e| e.get_value::<u32>(&OWNER_ENTITY_KEY))
                                        .map(|h| ehandle_to_index(h) as i32)
                                        .and_then(|owner_idx| resolve_pawn_to_slot(owner_idx, ctx));
                                    if owner_slot.is_none() {
                                        warn!(
                                            "[parse_replay] sinner death: non-pawn attacker index {} owner chain failed",
                                            attacker_index
                                        );
                                    }
                                    owner_slot
                                }
                                None => None,
                            };

                            if let Some(slot) = killer_slot {
                                self.sinner_tracker.record_sinner_death_killer(
                                    entity.index(),
                                    slot,
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn on_cmd(
        &mut self,
        _ctx: &Context,
        _cmd_header: &CmdHeader,
        _data: &[u8],
    ) -> Result<(), ParseError> {
        Ok(())
    }

    async fn on_packet(
        &mut self,
        ctx: &Context,
        packet_type: u32,
        data: &[u8],
    ) -> Result<(), ParseError> {
        if packet_type == CitadelUserMessageIds::KEUserMsgDamage as u32 {
            let msg = CCitadelUserMessageDamage::decode(data)?;
            let entities = ctx.entities().unwrap();

            let attacker_idx = msg.entindex_attacker();
            let victim_idx = msg.entindex_victim();
            let damage = msg.damage();
            let match_time_s = self.current_match_second;

            let (Some(attacker), Some(victim)) =
                (entities.get(&attacker_idx), entities.get(&victim_idx))
            else {
                return Ok(());
            };

            // Victim is a boss -- record health sample.
            if self.boss_tracker.is_boss_entity(victim.serializer().serializer_name.hash) {
                self.boss_tracker.record_boss_damage(victim_idx, ctx, match_time_s)?;
            }

            // Victim is the mid-boss -- record a health sample for fight window tracking.
            if Some(victim_idx) == self.mid_boss_tracker.mid_boss_entity_index() {
                if let Some(health) = victim.get_value::<i32>(&HEALTH_KEY) {
                    let match_time_s_f = (ctx.tick() as f32 * ctx.tick_interval())
                        - self.match_start_time_s.map(|s| s as f32).unwrap_or(0.0);
                    self.mid_boss_tracker.record_damage(health, match_time_s_f);
                }
            }

            // Victim is a sinner -- track last attacker for kill attribution, and append
            // a Dealt event when the attacker resolves to a player slot. Non-player
            // attackers (troopers, unowned NPCs) still get kill attribution via
            // record_damage but produce no Dealt event.
            if self.sinner_tracker.is_tracked_sinner(victim_idx) {
                self.sinner_tracker.record_damage(victim_idx, attacker_idx);
                if let Some(slot) = self.resolve_attacker_to_player_slot(attacker, ctx) {
                    self.sinner_tracker.record_dealt_event(
                        victim_idx,
                        slot,
                        damage,
                        match_time_s,
                    );
                }
            }

            // Attacker is a sinner -- append a Retaliated event. Confirmed
            // (lil-helper-sinner-interaction.md spike, replay 55841493): retaliation
            // always targets a CCitadelPlayerPawn directly.
            if self.sinner_tracker.is_tracked_sinner(attacker_idx)
                && victim.serializer_name_heq(CCITADELPLAYERPAWN_ENTITY)
                && let Some(slot) = resolve_pawn_to_slot(victim_idx, ctx)
            {
                self.sinner_tracker.record_retaliation(
                    attacker_idx,
                    slot,
                    damage,
                    match_time_s,
                );
            }

            let record = DamageRecord {
                damage,
                pre_damage: msg.pre_damage(),
                damage_type: msg.r#type(),
                citadel_type: msg.citadel_type(),
                entindex_inflictor: msg.entindex_inflictor(),
                entindex_ability: msg.entindex_ability(),
                damage_absorbed: msg.damage_absorbed(),
                victim_health_max: msg.victim_health_max(),
                victim_health_new: msg.victim_health_new(),
                flags: msg.flags(),
                ability_id: msg.ability_id(),
                attacker_class: msg.attacker_class(),
                victim_class: msg.victim_class(),
                victim_shield_max: msg.victim_shield_max(),
                victim_shield_new: msg.victim_shield_new(),
                hits: msg.hits(),
                health_lost: msg.health_lost(),
            };

            if let Err(error) = self.push_damage_record(ctx, attacker, victim, record) {
                error!("[parse_replay] Failed to push damage record: {:?}", error);
                return Err(error.into());
            }
        }

        // MidBossSpawned (ID 349) -- empty message, detected by packet_type only.
        // Not in the CitadelUserMessageIds enum -- use the raw integer.
        if packet_type == CitadelUserMessageIds::KEUserMsgMidBossSpawned as u32 {
            let match_time_s_f = (ctx.tick() as f32 * ctx.tick_interval())
                - self.match_start_time_s.map(|s| s as f32).unwrap_or(0.0);
            // Decode to validate no unexpected data (empty message struct)
            let _msg = CCitadelUserMsgMidBossSpawned::decode(data)?;
            self.mid_boss_tracker.handle_spawn(match_time_s_f);
        }

        // BossKilled (ID 347) -- filter to mid-boss via entity_killed_class == MID_BOSS_CLASS_ID.
        if packet_type == CitadelUserMessageIds::KEUserMsgBossKilled as u32 {
            let msg = CCitadelUserMsgBossKilled::decode(data)?;
            if msg.entity_killed_class == Some(MID_BOSS_CLASS_ID) {
                let pos = msg.entity_position.as_ref();
                // gametime is in demo-time coordinates (seconds since map load),
                // same as m_flGameStartTime. Subtract match_start_time_s to get
                // match-relative time. # TODO: verify with real replay data
                let matchtime_s = msg.gametime.unwrap_or(0.0)
                    - self.match_start_time_s.map(|s| s as f32).unwrap_or(0.0);
                self.mid_boss_tracker.handle_kill(
                    msg.objective_team.unwrap_or(0),
                    matchtime_s,
                    pos.and_then(|p| p.x).unwrap_or(0.0),
                    pos.and_then(|p| p.y).unwrap_or(0.0),
                    pos.and_then(|p| p.z).unwrap_or(0.0),
                    msg.bosses_remaining.unwrap_or(0),
                );
            }
        }

        // RejuvStatus (ID 350) -- rejuvenation grant status for mid-boss kill claiming.
        if packet_type == CitadelUserMessageIds::KEUserMsgRejuvStatus as u32 {
            let msg = CCitadelUserMsgRejuvStatus::decode(data)?;
            let match_time_s_f = (ctx.tick() as f32 * ctx.tick_interval())
                - self.match_start_time_s.map(|s| s as f32).unwrap_or(0.0);
            self.mid_boss_tracker.handle_rejuv_status(
                match_time_s_f,
                msg.player_pawn.unwrap_or(0),
                msg.user_team.unwrap_or(0),
                msg.killing_team.unwrap_or(0),
                msg.event_type.unwrap_or(0),
            );
        }

        // Handle post-match details (damage matrix)
        if packet_type == CitadelUserMessageIds::KEUserMsgPostMatchDetails as u32 {
            let details_msg = CCitadelUserMsgPostMatchDetails::decode(data)?;
            if let Some(bytes) = details_msg.match_details {
                let mut cursor = std::io::Cursor::new(bytes);
                if let Ok(meta) = CMsgMatchMetaDataContentsPatched::decode(&mut cursor) {
                    if let Some(match_info) = meta.match_info {
                        if let Some(damage_matrix) = match_info.damage_matrix {
                            debug!(
                                "[parse_replay] PostMatch damage_matrix: dealers={} samples={}",
                                damage_matrix.damage_dealers.len(),
                                damage_matrix.sample_time_s.len()
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Parse a replay file and return match data as JSON
pub async fn parse_replay(replay_full_path: &str) -> Result<serde_json::Value> {
    // Get file size for logging
    let file_size = fs::metadata(replay_full_path).map(|m| m.len()).unwrap_or(0);
    info!(
        "[parse_replay] Starting parse of {} ({} bytes)",
        replay_full_path, file_size
    );

    let file = File::open(replay_full_path)?;
    let buf_reader = BufReader::new(file);

    info!("[parse_replay] Opening demo file stream...");
    let demo_file = DemoFile::start_reading(buf_reader).map_err(|e| {
        error!("[parse_replay] Failed to open demo file stream: {:?}", e);
        anyhow::anyhow!("Failed to open demo file stream: {:?}", e)
    })?;
    info!("[parse_replay] Demo file stream opened successfully");

    let mut visitor = MyVisitor::default();
    info!("[parse_replay] Creating parser with visitor...");
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor).map_err(|e| {
        error!("[parse_replay] Failed to create parser: {:?}", e);
        anyhow::anyhow!("Failed to create parser: {:?}", e)
    })?;
    info!("[parse_replay] Parser created successfully");

    info!("[parse_replay] Running parser to end of demo...");
    parser.run_to_end().await.map_err(|e| {
        error!("[parse_replay] Parser failed during run_to_end: {:?}", e);
        anyhow::anyhow!("Parser failed during run_to_end: {:?}", e)
    })?;

    info!(
        "[parse_replay] Parse complete. match_start_time_s={:?}, match_duration_s={}, players={}, position_frames={}",
        visitor.match_start_time_s,
        visitor.current_match_second,
        visitor.players.len(),
        visitor.positions.len()
    );

    visitor.finalize_trackers();

    Ok(visitor.get_match_data_json())
}

#[cfg(test)]
mod tests;
