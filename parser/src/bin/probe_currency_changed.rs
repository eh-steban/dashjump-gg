//! Diagnostic binary: probe CCitadelUserMessage_CurrencyChanged and CCitadelUserMsgGoldHistory.
//!
//! Runs a full replay parse and logs, for each message:
//! - currency_type value distribution (what values appear?)
//! - currency_source distribution per currency_type
//! - delta range (positive = earn, negative = spend)
//! - accumulated balance per player (since new_value is absent from this message)
//! - per-second balance and earn rate for the first player seen
//! - GoldHistory per-minute records (as a cross-reference)
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
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::deadlock::{
    CitadelUserMessageIds,
    CCitadelUserMessageCurrencyChanged,
    CCitadelUserMsgGoldHistory,
};
use prost::Message;

/// Thin error wrapper so the Visitor::Error bound (`std::error::Error + Send + Sync + 'static`)
/// is satisfied. Internal helpers use anyhow::Result; the From impl lets `?` convert.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

const DEADLOCK_GAMERULES_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");
const CCITADELPLAYERPAWN_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelPlayerPawn");

const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);
const OWNER_ENTITY_KEY: u64 = fkey_from_path(&["m_hOwnerEntity"]);
const LOBBY_PLAYER_SLOT_KEY: u64 = fkey_from_path(&["m_unLobbyPlayerSlot"]);
const PLAYER_NAME_KEY: u64 = fkey_from_path(&["m_iszPlayerName"]);

// Per-player accumulated balance (since CurrencyChanged has no new_value field)
#[derive(Debug, Default, Clone)]
struct PlayerState {
    name: String,
    lobby_slot: u32,
    balance: i64,                      // accumulated from deltas
    balance_timeline: Vec<(u32, i64)>, // (match_sec, balance) -- one entry per second
    earn_timeline: Vec<(u32, i64)>,    // (match_sec, souls_earned) -- positive deltas only
    earn_this_sec: i64,                // accumulates within each second
    last_sec: u32,
}

struct ProbeVisitor {
    match_start_time_s: Option<f32>,

    // entity_index -> PlayerState
    players: HashMap<i32, PlayerState>,
    // pawn entity_index -> lobby_slot (cache for lookup)
    pawn_to_slot: HashMap<i32, u32>,
    // lobby_slot -> entity_index (reverse)
    slot_to_pawn: HashMap<u32, i32>,

    // currency_type -> count
    currency_type_tally: HashMap<i32, u64>,

    // (currency_type, currency_source) -> count
    type_source_tally: HashMap<(i32, i32), u64>,

    // delta distributions
    positive_deltas: Vec<i32>,  // earn events
    negative_deltas: Vec<i32>,  // spend events

    // GoldHistory per-minute reconciliation
    gold_history_minutes: HashMap<i32, Vec<(i32, i32)>>, // minute -> [(source, gold)]

    total_currency_events: u64,
    total_gold_history_events: u64,
}

impl Default for ProbeVisitor {
    fn default() -> Self {
        Self {
            match_start_time_s: None,
            players: HashMap::new(),
            pawn_to_slot: HashMap::new(),
            slot_to_pawn: HashMap::new(),
            currency_type_tally: HashMap::new(),
            type_source_tally: HashMap::new(),
            positive_deltas: Vec::new(),
            negative_deltas: Vec::new(),
            gold_history_minutes: HashMap::new(),
            total_currency_events: 0,
            total_gold_history_events: 0,
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

    fn resolve_pawn_to_slot(&mut self, entindex: i32, ctx: &Context) -> Option<u32> {
        if let Some(&slot) = self.pawn_to_slot.get(&entindex) {
            return Some(slot);
        }
        let entities = ctx.entities()?;
        let pawn = entities.get(&entindex)?;

        let owner_handle: u32 = pawn.get_value(&OWNER_ENTITY_KEY)?;
        let controller_index = ehandle_to_index(owner_handle);
        let controller = entities.get(&controller_index)?;
        let slot: u32 = controller.get_value(&LOBBY_PLAYER_SLOT_KEY)?;
        let name: String = controller
            .get_value::<Box<[u8]>>(&PLAYER_NAME_KEY)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();

        self.pawn_to_slot.insert(entindex, slot);
        self.slot_to_pawn.insert(slot, entindex);
        self.players.entry(entindex).or_insert_with(|| PlayerState {
            name,
            lobby_slot: slot,
            ..Default::default()
        });

        Some(slot)
    }

    fn handle_currency_changed(&mut self, msg: CCitadelUserMessageCurrencyChanged, ctx: &Context) {
        // Note: entindex_hero_pawn was removed from this proto; userid is now the player key.
        let player_userid = msg.userid.unwrap_or(-1);
        if player_userid < 0 {
            return;
        }

        let match_sec = match self.current_match_sec(ctx) {
            Some(s) => s,
            None => return,
        };

        let currency_type = msg.currency_type.unwrap_or(-1);
        let currency_source = msg.currency_source.unwrap_or(-1);
        let delta = msg.delta.unwrap_or(0);

        *self.currency_type_tally.entry(currency_type).or_default() += 1;
        *self.type_source_tally.entry((currency_type, currency_source)).or_default() += 1;

        if delta > 0 {
            self.positive_deltas.push(delta);
        } else if delta < 0 {
            self.negative_deltas.push(delta);
        }

        self.total_currency_events += 1;

        // Accumulate balance keyed by userid (entity-based pawn lookup no longer available)
        let player = self.players.entry(player_userid).or_insert_with(|| PlayerState {
            name: format!("userid:{}", player_userid),
            lobby_slot: player_userid as u32,
            ..Default::default()
        });
        player.balance += delta as i64;

        // Per-second earn accumulation
        if match_sec != player.last_sec {
            // Flush previous second
            if player.last_sec > 0 || !player.earn_timeline.is_empty() {
                player.earn_timeline.push((player.last_sec, player.earn_this_sec));
                player.balance_timeline.push((player.last_sec, player.balance - delta as i64));
            }
            player.earn_this_sec = 0;
            player.last_sec = match_sec;
        }

        if delta > 0 {
            player.earn_this_sec += delta as i64;
        }
    }

    fn handle_gold_history(&mut self, msg: CCitadelUserMsgGoldHistory) {
        self.total_gold_history_events += 1;

        for minute_rec in &msg.minute_records {
            let minute = minute_rec.match_minute.unwrap_or(-1);
            let records = self.gold_history_minutes.entry(minute).or_default();
            for gold_rec in &minute_rec.gold_records {
                let source = gold_rec.currency_source.unwrap_or(-1);
                let gold = gold_rec.gold.unwrap_or(0);
                records.push((source, gold));
            }
        }
    }
}

impl Visitor for &mut ProbeVisitor {
    type Error = ProbeError;

    async fn on_entity(&mut self, _ctx: &Context, _delta_header: DeltaHeader, entity: &Entity) -> Result<(), ProbeError> {
        if entity.serializer().serializer_name.hash == DEADLOCK_GAMERULES_ENTITY {
            if let Ok(t) = entity.try_get_value::<f32>(&GAME_START_KEY) {
                if t > 0.001 && self.match_start_time_s.is_none() {
                    self.match_start_time_s = Some(t);
                    println!("GAME_START detected: {:.3}s into replay", t);
                }
            }
        }
        Ok(())
    }

    async fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8]) -> Result<(), ProbeError> {
        if packet_type == CitadelUserMessageIds::KEUserMsgCurrencyChanged as u32 {
            let msg = CCitadelUserMessageCurrencyChanged::decode(data)
                .map_err(|e| ProbeError(anyhow::anyhow!(e)))?;
            self.handle_currency_changed(msg, ctx);
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgGoldHistory as u32 {
            let msg = CCitadelUserMsgGoldHistory::decode(data)
                .map_err(|e| ProbeError(anyhow::anyhow!(e)))?;
            self.handle_gold_history(msg);
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, _ctx: &Context) -> Result<(), ProbeError> { Ok(()) }
    async fn on_cmd(&mut self, _ctx: &Context, _cmd_header: &CmdHeader, _data: &[u8]) -> Result<(), ProbeError> { Ok(()) }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args().nth(1).expect("Usage: probe_currency_changed <replay.dem>");

    eprintln!("Probing: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = ProbeVisitor::default();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    // ========== SUMMARY ==========
    println!("\n========== CURRENCY CHANGED SUMMARY ==========");
    println!("Total CurrencyChanged events:  {}", visitor.total_currency_events);
    println!("Total GoldHistory events:      {}", visitor.total_gold_history_events);
    println!("Players resolved:              {}", visitor.players.len());

    println!("\n-- currency_type value distribution --");
    let mut type_vals: Vec<(i32, u64)> = visitor.currency_type_tally.iter().map(|(&k, &v)| (k, v)).collect();
    type_vals.sort_by_key(|&(v, _)| v);
    for (t, count) in &type_vals {
        println!("  currency_type={:>4} : {} events", t, count);
    }

    println!("\n-- (currency_type, currency_source) distribution --");
    let mut ts_vals: Vec<((i32, i32), u64)> = visitor.type_source_tally.iter().map(|(&k, &v)| (k, v)).collect();
    ts_vals.sort_by_key(|&((t, s), _)| (t, s));
    for ((t, s), count) in &ts_vals {
        println!("  type={:>4} source={:>4} : {} events", t, s, count);
    }

    println!("\n-- positive delta (earn) distribution --");
    if !visitor.positive_deltas.is_empty() {
        let min = visitor.positive_deltas.iter().copied().min().unwrap();
        let max = visitor.positive_deltas.iter().copied().max().unwrap();
        let sum: i64 = visitor.positive_deltas.iter().map(|&v| v as i64).sum();
        let count = visitor.positive_deltas.len();
        println!("  count={} min={} max={} mean={:.1}", count, min, max, sum as f64 / count as f64);

        // Bucket histogram (0-50, 50-100, 100-200, 200-500, 500+)
        let buckets = [(0, 50), (50, 100), (100, 200), (200, 500), (500, i32::MAX)];
        for (lo, hi) in &buckets {
            let n = visitor.positive_deltas.iter().filter(|&&v| v >= *lo && v < *hi).count();
            let label = if *hi == i32::MAX { format!("{}+", lo) } else { format!("{}-{}", lo, hi) };
            println!("  earn {:>10} : {} events", label, n);
        }
    }

    println!("\n-- negative delta (spend) distribution --");
    if !visitor.negative_deltas.is_empty() {
        let min = visitor.negative_deltas.iter().copied().min().unwrap();
        let max = visitor.negative_deltas.iter().copied().max().unwrap();
        let sum: i64 = visitor.negative_deltas.iter().map(|&v| v as i64).sum();
        let count = visitor.negative_deltas.len();
        println!("  count={} min={} max={} mean={:.1}", count, min, max, sum as f64 / count as f64);
    }

    println!("\n-- GoldHistory per-minute breakdown (first 5 minutes) --");
    let mut minutes: Vec<i32> = visitor.gold_history_minutes.keys().copied().collect();
    minutes.sort();
    for minute in minutes.iter().take(5) {
        let records = &visitor.gold_history_minutes[minute];
        let total: i32 = records.iter().map(|(_, g)| g).sum();
        println!("  minute={} total_gold={} ({} source records)", minute, total, records.len());
        let mut by_source: HashMap<i32, i32> = HashMap::new();
        for (src, gold) in records {
            *by_source.entry(*src).or_default() += gold;
        }
        let mut src_list: Vec<(i32, i32)> = by_source.into_iter().collect();
        src_list.sort_by_key(|(s, _)| *s);
        for (src, gold) in src_list {
            println!("    source={:>4} : {}", src, gold);
        }
    }

    println!("\n-- Per-player balance summary --");
    let mut player_list: Vec<&PlayerState> = visitor.players.values().collect();
    player_list.sort_by_key(|p| p.lobby_slot);
    for player in &player_list {
        println!("  slot={} name=\"{}\" final_balance={}", player.lobby_slot, player.name, player.balance);
    }

    // Emit per-second sample for the player with the most events (first by slot)
    if let Some(sample_player) = player_list.first() {
        println!("\n-- Per-second sample: slot={} name=\"{}\" (first 60s) --", sample_player.lobby_slot, sample_player.name);
        println!("  {:>6}  {:>12}  {:>12}", "sec", "balance", "earn_this_sec");

        // Build a dense timeline with carry-forward for balance
        let max_sec = sample_player.balance_timeline.last().map(|(s, _)| *s).unwrap_or(60).min(60);
        let mut last_balance = 0i64;
        let earn_map: HashMap<u32, i64> = sample_player.earn_timeline.iter().copied().collect();
        let balance_map: HashMap<u32, i64> = sample_player.balance_timeline.iter().copied().collect();

        for sec in 0..=max_sec {
            if let Some(&bal) = balance_map.get(&sec) {
                last_balance = bal;
            }
            let earn = earn_map.get(&sec).copied().unwrap_or(0);
            println!("  {:>6}  {:>12}  {:>12}", sec, last_balance, earn);
        }
    }

    Ok(())
}
