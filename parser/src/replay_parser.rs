//! Main replay parser - coordinates parsing of Deadlock demo files

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity, ehandle_to_index, fkey_from_path};
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::deadlock::{
    CCitadelUserMessageDamage, CCitadelUserMsgPostMatchDetails, CMsgMatchMetaDataContents,
    CitadelUserMessageIds,
};
use haste::valveprotos::prost::Message;
use tracing::{error, info};

use crate::domain::{DamageRecord, Player, PlayerPosition};
use crate::entities::constants::*;
use crate::tracking::{BossTracker, CreepTracker};
use crate::utils::{get_entity_position, get_steam_id32};

/// Main visitor that collects all match data during parsing
#[derive(Debug)]
struct MyVisitor {
    total_match_time_s: u32,
    match_start_time_s: Option<u32>,
    damage_window: HashMap<u32, HashMap<u32, Vec<DamageRecord>>>,
    damage: Vec<HashMap<u32, HashMap<u32, Vec<DamageRecord>>>>,
    players: Vec<Player>,
    entity_name_hash_to_player_slot: HashMap<u32, u32>,
    positions_window: Vec<PlayerPosition>,
    positions: Vec<Vec<PlayerPosition>>,
    boss_tracker: BossTracker,
    creep_tracker: CreepTracker,
    lane_data_updated: bool,
}

impl Default for MyVisitor {
    fn default() -> Self {
        let boss_tracker = BossTracker::new();
        let creep_tracker = CreepTracker::new(boss_tracker.lane_key());
        Self {
            total_match_time_s: 0,
            match_start_time_s: None,
            damage_window: HashMap::new(),
            damage: Vec::new(),
            players: Vec::new(),
            entity_name_hash_to_player_slot: HashMap::new(),
            positions_window: Vec::new(),
            positions: Vec::new(),
            boss_tracker,
            creep_tracker,
            lane_data_updated: false,
        }
    }
}

impl MyVisitor {
    /// Get final match data as JSON
    pub fn get_match_data_json(&self) -> serde_json::Value {
        let creep_waves = self.creep_tracker.get_output();

        serde_json::json!({
            "total_match_time_s": self.total_match_time_s,
            "match_start_time_s": self.match_start_time_s,
            "damage": self.damage,
            "players": self.players,
            "positions": self.positions,
            "bosses": self.boss_tracker.get_output(),
            "creep_waves": creep_waves,
        })
    }

    /// Check if lane swap is locked and update player lane/color
    fn check_and_update_lane_lock(&mut self, entity: &Entity) -> Result<()> {
        if entity.serializer().serializer_name.str.as_ref() != "CCitadelPlayerController" {
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

                // TODO: Lane color fix - need to capture zipline_lane_color at lock time
                // Currently captured at player discovery which may be incorrect
                // See ZIPLINE_LANE_COLOR_KEY constant in entities/constants.rs

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
                | CNPC_MIDBOSS_ENTITY
                | CITEMXP_ENTITY
                | CCITADEL_DESTROYABLE_BUILDING_ENTITY
                | CNPC_BARRACKBOSS_ENTITY
                | CNPC_BOSS_TIER2_ENTITY
                | CNPC_BOSS_TIER3_ENTITY
                | CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY
                | CNPC_BASE_DEFENSE_SENTRY_ENTITY
                | CNPC_SHIELDEDSENTRY_ENTITY
        )
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
                        zipline_lane_color: owner_entity
                            .get_value(&ZIPLINE_LANE_COLOR_KEY)
                            .unwrap_or(999999),
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
            CNPC_MIDBOSS_ENTITY => 24,
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
                "Unknown entity - Name: {}, Hash: {}",
                serializer_entity_name.str, serializer_entity_name.hash
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
}

impl Visitor for &mut MyVisitor {
    fn on_tick_end(&mut self, ctx: &Context) -> Result<()> {
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
            self.boss_tracker.build_health_window(match_window);
            self.creep_tracker.build_wave_window(match_window);

            // Collect positions for all tracked entities
            for (_index, entity) in ctx.entities().unwrap().iter() {
                if !self.should_track_position(entity) {
                    continue;
                }
                let position = get_entity_position(entity);
                let custom_id = self.get_custom_id(ctx, entity);
                let is_npc = self.is_npc_entity(entity);

                self.positions_window.push(PlayerPosition {
                    custom_id: custom_id.to_string(),
                    x: position[0],
                    y: position[1],
                    z: position[2],
                    is_npc,
                });
            }

            self.damage.push(std::mem::take(&mut self.damage_window));
            self.positions
                .push(std::mem::take(&mut self.positions_window));
            if this_window == 0 {
                self.total_match_time_s = this_window;
            } else {
                self.total_match_time_s = this_window - 1;
            }
        }

        Ok(())
    }

    fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<()> {
        // Handle game rules for match start time
        if entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY) {
            self.handle_game_rules(entity, ctx.tick())?;
        }

        // Track boss and creep lifecycle
        let hash = entity.serializer().serializer_name.hash;

        // Only track creeps after match has started to avoid pre-match entities
        let match_started = self.match_start_time_s.is_some();

        match delta_header {
            DeltaHeader::CREATE => {
                if self.boss_tracker.is_boss_entity(hash) {
                    let custom_id = self.get_custom_id(ctx, entity);
                    let match_time_s = self.total_match_time_s.saturating_sub(self.match_start_time_s.unwrap_or(0));
                    self.boss_tracker.handle_boss_create(
                        entity,
                        custom_id,
                        hash,
                        match_time_s,
                    );
                }
                // Only track creeps after match starts
                if match_started && CreepTracker::is_creep_entity(hash) {
                    self.creep_tracker.handle_creep_create(entity);
                }
            }
            DeltaHeader::DELETE => {
                let match_time_s = self.total_match_time_s.saturating_sub(self.match_start_time_s.unwrap_or(0));
                self.boss_tracker
                    .handle_boss_delete(entity, match_time_s);
                if CreepTracker::is_creep_entity(hash) {
                    self.creep_tracker.handle_creep_delete(entity.index());
                }
            }
            DeltaHeader::UPDATE => {
                // Check for lane lock updates
                if !self.lane_data_updated {
                    self.check_and_update_lane_lock(entity)?;
                }
                // Update creep positions (only if match started)
                if match_started && CreepTracker::is_creep_entity(hash) {
                    self.creep_tracker.handle_creep_update(entity);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn on_cmd(&mut self, _ctx: &Context, _cmd_header: &CmdHeader, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8]) -> Result<()> {
        if packet_type == CitadelUserMessageIds::KEUserMsgDamage as u32 {
            let msg = CCitadelUserMessageDamage::decode(data)?;
            let entities = ctx.entities().unwrap();

            let attacker = entities.get(&msg.entindex_attacker());
            let victim = entities.get(&msg.entindex_victim());

            if attacker.is_none() || victim.is_none() {
                return Ok(());
            }

            // Check if victim is a boss and record health sample
            let victim_hash = victim.unwrap().serializer().serializer_name.hash;
            if self.boss_tracker.is_boss_entity(victim_hash) {
                let match_time_s = self.total_match_time_s.saturating_sub(self.match_start_time_s.unwrap_or(0));
                self.boss_tracker.record_boss_damage(
                    msg.entindex_victim(),
                    ctx,
                    match_time_s,
                )?;
            }

            let record = DamageRecord {
                damage: msg.damage(),
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

            if let Err(error) =
                self.push_damage_record(ctx, attacker.unwrap(), victim.unwrap(), record)
            {
                println!("Failed to push damage record: {:?}", error);
                return Err(error);
            }
        }

        // Handle post-match details (damage matrix)
        if packet_type == CitadelUserMessageIds::KEUserMsgPostMatchDetails as u32 {
            let details_msg = CCitadelUserMsgPostMatchDetails::decode(data)?;
            if let Some(bytes) = details_msg.match_details {
                let mut cursor = std::io::Cursor::new(bytes);
                if let Ok(meta) = CMsgMatchMetaDataContents::decode(&mut cursor) {
                    if let Some(match_info) = meta.match_info {
                        if let Some(damage_matrix) = match_info.damage_matrix {
                            println!(
                                "PostMatch damage_matrix: dealers={} samples={}",
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
pub fn parse_replay(replay_full_path: &str) -> Result<serde_json::Value> {
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
    parser.run_to_end().map_err(|e| {
        error!("[parse_replay] Parser failed during run_to_end: {:?}", e);
        anyhow::anyhow!("Parser failed during run_to_end: {:?}", e)
    })?;

    info!(
        "[parse_replay] Parse complete. match_start_time_s={:?}, total_match_time_s={}, players={}, position_frames={}",
        visitor.match_start_time_s,
        visitor.total_match_time_s,
        visitor.players.len(),
        visitor.positions.len()
    );

    Ok(visitor.get_match_data_json())
}
