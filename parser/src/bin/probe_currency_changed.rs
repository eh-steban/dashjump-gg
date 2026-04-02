//! Probe: validate CCitadelPlayerPawn entity field access for souls tracking.
//!
//! Subscribes to CCitadelPlayerPawn entities and reads m_PlayerDataGlobal.m_iGoldNetWorth
//! for each player. In on_tick_end, accumulates one balance sample per second per player
//! for match seconds [480, 490). After parsing, prints per-second balances for that window
//! and total unique players seen.
//!
//! Usage (from repo root):
//!   docker-compose run --rm dashjump-parser cargo run --bin probe_currency_changed -- /parser/src/replays/55423930_379917638.dem

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity, ehandle_to_index, fkey_from_path};
use haste::parser::{Context, Parser, Visitor};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

const DEADLOCK_GAMERULES_ENTITY: u64 = haste::fxhash::hash_bytes(b"CCitadelGameRulesProxy");
const CCITADELPLAYERPAWN_ENTITY: u64 = haste::fxhash::hash_bytes(b"CCitadelPlayerPawn");

const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);
const OWNER_ENTITY_KEY: u64 = fkey_from_path(&["m_hOwnerEntity"]);
const LOBBY_PLAYER_SLOT_KEY: u64 = fkey_from_path(&["m_unLobbyPlayerSlot"]);
const GOLD_NET_WORTH_KEY: u64 = fkey_from_path(&["m_PlayerDataGlobal", "m_iGoldNetWorth"]);
const GOLD_NET_WORTH_FLAT_KEY: u64 = fkey_from_path(&["m_iGoldNetWorth"]);

/// One balance sample for a player at a given match second.
#[derive(Debug, Clone)]
struct BalanceSample {
    player_slot: u32,
    match_sec: u32,
    balance: i32,
    used_flat_key: bool,
}

struct ProbeVisitor {
    match_start_time_s: Option<f32>,
    // entity_index -> (player_slot, latest balance, used_flat_key)
    pawn_state: HashMap<i32, (u32, i32, bool)>,
    // (match_sec, entity_index) -> BalanceSample  -- one sample per second per player
    samples: HashMap<(u32, i32), BalanceSample>,
    last_sampled_sec: u32,
}

impl Default for ProbeVisitor {
    fn default() -> Self {
        Self {
            match_start_time_s: None,
            pawn_state: HashMap::new(),
            samples: HashMap::new(),
            last_sampled_sec: u32::MAX,
        }
    }
}

impl ProbeVisitor {
    fn current_match_sec(&self, ctx: &Context) -> Option<u32> {
        let start = self.match_start_time_s?;
        let elapsed = (ctx.tick() as f32) * ctx.tick_interval() - start;
        if elapsed < 0.0 {
            return None;
        }
        Some(elapsed as u32)
    }

    fn resolve_player_slot(ctx: &Context, entity: &Entity) -> Option<u32> {
        let owner_handle: u32 = entity.get_value(&OWNER_ENTITY_KEY)?;
        let controller_index = ehandle_to_index(owner_handle);
        let controller = ctx.entities()?.get(&controller_index)?;
        controller.get_value(&LOBBY_PLAYER_SLOT_KEY)
    }
}

impl Visitor for &mut ProbeVisitor {
    type Error = ProbeError;

    async fn on_entity(
        &mut self,
        ctx: &Context,
        _delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), ProbeError> {
        if entity.serializer().serializer_name.hash == DEADLOCK_GAMERULES_ENTITY {
            if let Ok(t) = entity.try_get_value::<f32>(&GAME_START_KEY) {
                if t > 0.001 && self.match_start_time_s.is_none() {
                    self.match_start_time_s = Some(t);
                    println!("GAME_START detected: {:.3}s into replay", t);
                }
            }
        }

        if entity.serializer().serializer_name.hash != CCITADELPLAYERPAWN_ENTITY {
            return Ok(());
        }

        let Some(player_slot) = ProbeVisitor::resolve_player_slot(ctx, entity) else {
            return Ok(());
        };

        let (balance, used_flat) = if let Some(b) = entity.get_value::<i32>(&GOLD_NET_WORTH_KEY) {
            (b, false)
        } else if let Some(b) = entity.get_value::<i32>(&GOLD_NET_WORTH_FLAT_KEY) {
            (b, true)
        } else {
            return Ok(());
        };

        self.pawn_state.insert(entity.index(), (player_slot, balance, used_flat));

        Ok(())
    }

    async fn on_tick_end(&mut self, ctx: &Context) -> Result<(), ProbeError> {
        let Some(match_sec) = self.current_match_sec(ctx) else {
            return Ok(());
        };

        if !(480..490).contains(&match_sec) {
            return Ok(());
        }

        if match_sec == self.last_sampled_sec {
            return Ok(());
        }
        self.last_sampled_sec = match_sec;

        for (&entity_index, &(player_slot, balance, used_flat)) in &self.pawn_state {
            let key = (match_sec, entity_index);
            self.samples.entry(key).or_insert(BalanceSample {
                player_slot,
                match_sec,
                balance,
                used_flat_key: used_flat,
            });
        }

        Ok(())
    }

    async fn on_packet(
        &mut self,
        _ctx: &Context,
        _packet_type: u32,
        _data: &[u8],
    ) -> Result<(), ProbeError> {
        Ok(())
    }

    async fn on_cmd(
        &mut self,
        _ctx: &Context,
        _cmd_header: &CmdHeader,
        _data: &[u8],
    ) -> Result<(), ProbeError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args().nth(1).expect("Usage: probe_currency_changed <replay.dem>");

    eprintln!("Probing entity fields: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = ProbeVisitor::default();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    // Collect per-player per-second balances for seconds 480-489
    // player_slot -> sec -> BalanceSample
    let mut by_player: HashMap<u32, HashMap<u32, &BalanceSample>> = HashMap::new();
    for sample in visitor.samples.values() {
        by_player
            .entry(sample.player_slot)
            .or_default()
            .insert(sample.match_sec, sample);
    }

    println!("\n========== ENTITY FIELD PROBE: m_iGoldNetWorth (seconds 480-489) ==========");
    println!("Unique players seen: {}", by_player.len());

    let flat_key_count = visitor.samples.values().filter(|s| s.used_flat_key).count();
    let nested_key_count = visitor.samples.values().filter(|s| !s.used_flat_key).count();
    println!("Field key: nested={} flat={}", nested_key_count, flat_key_count);

    println!();
    println!("{:>6}  {:>6}  {:>8}  {:>6}", "slot", "sec", "balance", "key");

    let mut slots: Vec<u32> = by_player.keys().copied().collect();
    slots.sort();

    for slot in &slots {
        let sec_map = &by_player[slot];
        for sec in 480u32..490 {
            if let Some(sample) = sec_map.get(&sec) {
                let key_label = if sample.used_flat_key { "flat" } else { "nested" };
                println!(
                    "{:>6}  {:>6}  {:>8}  {:>6}",
                    slot, sec, sample.balance, key_label
                );
            } else {
                println!("{:>6}  {:>6}  {:>8}  {:>6}", slot, sec, "---", "---");
            }
        }
        println!();
    }

    Ok(())
}
