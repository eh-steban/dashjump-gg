// probe_trooper_minimap.rs
//
// Phase 2 sampler for the Trooper Minimap FOW Tracking discovery
// (`private/plans/discovery/trooper-minimap-fow-tracking.md`).
//
// Purpose
// -------
// Characterise the `CCitadelTrooperMinimap.m_vecFOWEntities` vector (192-slot
// `CUtlVectorEmbeddedNetworkVar<STrooperFOWEntity>`) and cross-reference it against
// the existing `CNPC_Trooper` subscription so the discovery checkpoint can answer:
//   - Q3  position precision / encoding of `m_nPositionXY: uint16`
//   - Q4  health fidelity -- STrooperFOWEntity carries no health field, so the
//         only fidelity signal is slot presence / absence. Emit slot occupancy.
//   - Q5  per-slot `m_nEntIndex` values -- make-or-break for join back to CNPC_Trooper
//   - Q7  slot lifecycle (stable indices / compaction / alive-flag toggle)
//   - Q8  cage / zipline-phase coverage (compare slot count pre-drop vs lane wave count)
//   - Q9  entity scope -- does m_nEntIndex ever point to a non-CNPC_Trooper entity?
//   - Q10 update-event volume (minimap entity callbacks + per-slot field deltas vs
//         CNPC_Trooper entity callbacks)
//   - Q11 position fidelity: paired minimap vs CNPC_Trooper position delta (done in
//         Phase 3 analysis; this probe supplies the raw pairs)
//
// NOT part of the parser's normal build
// -------------------------------------
// Source of truth for this file is `private/engineering/tools/probe_trooper_minimap.rs`.
// To execute:
//
//   1. Ensure the parser container is up:
//        docker compose up -d dashjump-parser
//   2. Copy the file into the parser crate:
//        cp private/engineering/tools/probe_trooper_minimap.rs \
//           parser/src/bin/probe_trooper_minimap.rs
//   3. Run with the target replay; redirect to the samples dir:
//        docker compose exec dashjump-parser cargo run --release \
//          --bin probe_trooper_minimap -- \
//          /parser/src/replays/68175583_527726523.dem \
//          > private/engineering/samples/trooper_minimap_68175583.jsonl
//   4. Delete the parser/src/bin/ copy:
//        rm parser/src/bin/probe_trooper_minimap.rs
//
// Dynamic-array key construction
// ------------------------------
// `CUtlVectorEmbeddedNetworkVar<T>` maps to haste's `DynamicSerializerArray`. The
// stored field key for `m_vecFOWEntities[idx].<subfield>` is:
//
//   base  = fxhash::hash_bytes("m_vecFOWEntities")
//   slot  = fxhash::add_u64_to_hash(0, idx as u64)
//   outer = fxhash::add_u64_to_hash(base, slot)
//   key   = fxhash::add_u64_to_hash(outer, fxhash::hash_bytes("<subfield>"))
//
// This matches the runtime key construction in haste_core::entities::Entity::parse
// (the `is_dynamic_array()` branch).
//
// Output format: one JSON object per sampled tick (tick % 60 == 0 after match start)
// plus a final `"kind": "summary"` line with cumulative counters.

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{
    DeltaHeader, Entity, deadlock_coord_from_cell, fkey_from_path,
};
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};

// ---------- entity hashes ----------

const CCITADEL_TROOPER_MINIMAP: u64 = fxhash::hash_bytes(b"CCitadelTrooperMinimap");
const CNPC_TROOPER: u64 = fxhash::hash_bytes(b"CNPC_Trooper");
const DEADLOCK_GAMERULES_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");

// ---------- scalar field keys ----------

const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);
const TIME_LAST_UPDATE_KEY: u64 = fkey_from_path(&["m_timeLastUpdate"]);
const NPC_LANE_KEY: u64 = fkey_from_path(&["m_iLane"]);
const NPC_TEAM_KEY: u64 = fkey_from_path(&["m_iTeamNum"]);
const NPC_HEALTH_KEY: u64 = fkey_from_path(&["m_iHealth"]);
const NPC_LIFE_STATE_KEY: u64 = fkey_from_path(&["m_lifeState"]);
const NPC_STATE_KEY: u64 = fkey_from_path(&["m_NPCState"]);
const NPC_CELL_X: u64 = fkey_from_path(&["CBodyComponent", "m_cellX"]);
const NPC_CELL_Y: u64 = fkey_from_path(&["CBodyComponent", "m_cellY"]);
const NPC_VEC_X: u64 = fkey_from_path(&["CBodyComponent", "m_vecX"]);
const NPC_VEC_Y: u64 = fkey_from_path(&["CBodyComponent", "m_vecY"]);

// ---------- dynamic-array helpers ----------

const MAX_SLOTS: usize = 192;
const ARRAY_NAME: &[u8] = b"m_vecFOWEntities";
const SUB_POS_XY: &[u8] = b"m_nPositionXY";
const SUB_ENT_IDX: &[u8] = b"m_nEntIndex";
const SUB_TEAM: &[u8] = b"m_nTeam";

const fn dyn_array_key(array_name: &[u8], idx: usize, subfield: &[u8]) -> u64 {
    let base = fxhash::hash_bytes(array_name);
    let slot = fxhash::add_u64_to_hash(0, idx as u64);
    let outer = fxhash::add_u64_to_hash(base, slot);
    fxhash::add_u64_to_hash(outer, fxhash::hash_bytes(subfield))
}

// ---------- probe state ----------

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

#[derive(Default, Clone, Copy)]
struct SlotSnapshot {
    pos_xy: Option<u16>,
    ent_idx: Option<u32>,
    team: Option<i8>,
}

impl SlotSnapshot {
    fn is_occupied(&self) -> bool {
        self.ent_idx.is_some() || self.pos_xy.is_some() || self.team.is_some()
    }
}

#[derive(Default, Clone, Copy)]
struct SlotCounters {
    pos_xy_updates: u64,
    ent_idx_updates: u64,
    team_updates: u64,
}

struct ProbeVisitor {
    match_start_time_s: Option<f32>,
    tick_interval: f32,

    // Last-seen values per slot (for delta detection).
    slot_values: [SlotSnapshot; MAX_SLOTS],
    // Cumulative update counts per slot subfield.
    slot_counters: [SlotCounters; MAX_SLOTS],

    // Precomputed keys per (slot, subfield).
    keys_pos_xy: [u64; MAX_SLOTS],
    keys_ent_idx: [u64; MAX_SLOTS],
    keys_team: [u64; MAX_SLOTS],

    // Callback counts (Q10 cost signal).
    minimap_on_entity_calls: u64,
    cnpc_trooper_on_entity_calls: u64,

    // Slot high-water mark (highest slot index observed occupied).
    max_slot_seen: i32,

    // Last-emitted tick (so we emit at most once per 60-tick boundary).
    last_sample_tick: Option<i32>,

    match_started: bool,
    first_create_tick: Option<i32>,
    last_seen_tick: Option<i32>,

    stdout: std::io::BufWriter<std::io::Stdout>,
}

fn build_keys_pos() -> [u64; MAX_SLOTS] {
    let mut arr = [0u64; MAX_SLOTS];
    let mut i = 0;
    while i < MAX_SLOTS {
        arr[i] = dyn_array_key(ARRAY_NAME, i, SUB_POS_XY);
        i += 1;
    }
    arr
}

fn build_keys_ent() -> [u64; MAX_SLOTS] {
    let mut arr = [0u64; MAX_SLOTS];
    let mut i = 0;
    while i < MAX_SLOTS {
        arr[i] = dyn_array_key(ARRAY_NAME, i, SUB_ENT_IDX);
        i += 1;
    }
    arr
}

fn build_keys_team() -> [u64; MAX_SLOTS] {
    let mut arr = [0u64; MAX_SLOTS];
    let mut i = 0;
    while i < MAX_SLOTS {
        arr[i] = dyn_array_key(ARRAY_NAME, i, SUB_TEAM);
        i += 1;
    }
    arr
}

impl ProbeVisitor {
    fn new() -> Self {
        Self {
            match_start_time_s: None,
            tick_interval: 1.0 / 64.0,
            slot_values: [SlotSnapshot::default(); MAX_SLOTS],
            slot_counters: [SlotCounters::default(); MAX_SLOTS],
            keys_pos_xy: build_keys_pos(),
            keys_ent_idx: build_keys_ent(),
            keys_team: build_keys_team(),
            minimap_on_entity_calls: 0,
            cnpc_trooper_on_entity_calls: 0,
            max_slot_seen: -1,
            last_sample_tick: None,
            match_started: false,
            first_create_tick: None,
            last_seen_tick: None,
            stdout: std::io::BufWriter::with_capacity(
                1 << 20,
                std::io::stdout(),
            ),
        }
    }

    fn match_time_s(&self, tick: i32) -> f32 {
        (tick as f32 * self.tick_interval) - self.match_start_time_s.unwrap_or(0.0)
    }

    /// Scan all slots in the minimap entity, bumping per-field counters on change.
    /// Updates the last-seen snapshot so subsequent calls diff against fresh state.
    fn scan_minimap_entity(&mut self, entity: &Entity) {
        for slot in 0..MAX_SLOTS {
            let pos: Option<u16> = entity.get_value(&self.keys_pos_xy[slot]);
            let ent: Option<u32> = entity.get_value(&self.keys_ent_idx[slot]);
            let team_raw: Option<i32> = entity.get_value(&self.keys_team[slot]);
            let team = team_raw.map(|v| v as i8);

            let prev = self.slot_values[slot];
            if pos != prev.pos_xy {
                self.slot_counters[slot].pos_xy_updates += 1;
            }
            if ent != prev.ent_idx {
                self.slot_counters[slot].ent_idx_updates += 1;
            }
            if team != prev.team {
                self.slot_counters[slot].team_updates += 1;
            }

            self.slot_values[slot] = SlotSnapshot {
                pos_xy: pos,
                ent_idx: ent,
                team,
            };

            if self.slot_values[slot].is_occupied() && slot as i32 > self.max_slot_seen {
                self.max_slot_seen = slot as i32;
            }
        }
    }

    /// Build the JSONL line for the current tick and flush it.
    fn emit_sample(&mut self, ctx: &Context) {
        use std::io::Write;
        let tick = ctx.tick();
        let matchtime_s = self.match_time_s(tick);

        // Minimap occupied slots.
        let mut slots_json = String::new();
        slots_json.push('[');
        let mut first = true;
        for slot in 0..MAX_SLOTS {
            let s = self.slot_values[slot];
            if !s.is_occupied() {
                continue;
            }
            if !first {
                slots_json.push(',');
            }
            first = false;
            let pos_xy = s.pos_xy.map(|v| v as i32).unwrap_or(-1);
            let ent = s.ent_idx.map(|v| v as i64).unwrap_or(-1);
            let team = s.team.map(|v| v as i32).unwrap_or(-999);
            let c = self.slot_counters[slot];
            slots_json.push_str(&format!(
                "{{\"slot\":{slot},\"pos_xy\":{pos_xy},\"ent_idx\":{ent},\"team\":{team},\
                \"upd_pos\":{},\"upd_ent\":{},\"upd_team\":{}}}",
                c.pos_xy_updates, c.ent_idx_updates, c.team_updates,
            ));
        }
        slots_json.push(']');

        // CNPC_Trooper census at this tick.
        let mut troopers_json = String::new();
        troopers_json.push('[');
        let mut tfirst = true;
        if let Some(entities) = ctx.entities() {
            for (idx, entity) in entities.iter() {
                if !entity.serializer_name_heq(CNPC_TROOPER) {
                    continue;
                }
                let lane: i32 = entity.get_value(&NPC_LANE_KEY).unwrap_or(-1);
                let team: u32 = entity.get_value(&NPC_TEAM_KEY).unwrap_or(0);
                let health: i32 = entity.get_value(&NPC_HEALTH_KEY).unwrap_or(-1);
                let life_state: u32 = entity.get_value(&NPC_LIFE_STATE_KEY).unwrap_or(99);
                let npc_state: i32 = entity.get_value(&NPC_STATE_KEY).unwrap_or(-1);

                let cell_x: u16 = entity.get_value(&NPC_CELL_X).unwrap_or(0);
                let cell_y: u16 = entity.get_value(&NPC_CELL_Y).unwrap_or(0);
                let vec_x: f32 = entity.get_value(&NPC_VEC_X).unwrap_or(0.0);
                let vec_y: f32 = entity.get_value(&NPC_VEC_Y).unwrap_or(0.0);
                let world_x = deadlock_coord_from_cell(cell_x, vec_x);
                let world_y = deadlock_coord_from_cell(cell_y, vec_y);

                if !tfirst {
                    troopers_json.push(',');
                }
                tfirst = false;
                troopers_json.push_str(&format!(
                    "{{\"idx\":{idx},\"lane\":{lane},\"team\":{team},\"health\":{health},\
                    \"life_state\":{life_state},\"npc_state\":{npc_state},\
                    \"world_x\":{world_x:.2},\"world_y\":{world_y:.2}}}"
                ));
            }
        }
        troopers_json.push(']');

        let line = format!(
            "{{\"kind\":\"sample\",\"tick\":{},\"matchtime_s\":{:.3},\
            \"minimap_on_entity_calls\":{},\"cnpc_trooper_on_entity_calls\":{},\
            \"max_slot_seen\":{},\"minimap_slots\":{},\"cnpc_troopers\":{}}}\n",
            tick,
            matchtime_s,
            self.minimap_on_entity_calls,
            self.cnpc_trooper_on_entity_calls,
            self.max_slot_seen,
            slots_json,
            troopers_json,
        );
        let _ = self.stdout.write_all(line.as_bytes());
    }

    fn emit_summary(&mut self) {
        use std::io::Write;
        let total_pos: u64 = self.slot_counters.iter().map(|c| c.pos_xy_updates).sum();
        let total_ent: u64 = self.slot_counters.iter().map(|c| c.ent_idx_updates).sum();
        let total_team: u64 = self.slot_counters.iter().map(|c| c.team_updates).sum();
        let total_field_deltas = total_pos + total_ent + total_team;

        let occupied_slots = self
            .slot_values
            .iter()
            .filter(|s| s.is_occupied())
            .count();

        let line = format!(
            "{{\"kind\":\"summary\",\"match_start_time_s\":{},\
            \"tick_interval\":{:.6},\"last_seen_tick\":{},\
            \"first_minimap_create_tick\":{},\
            \"minimap_on_entity_calls\":{},\"cnpc_trooper_on_entity_calls\":{},\
            \"minimap_slot_field_deltas_total\":{},\
            \"delta_breakdown\":{{\"pos_xy\":{},\"ent_idx\":{},\"team\":{}}},\
            \"max_slot_seen\":{},\"occupied_slots_end_of_match\":{}}}\n",
            self.match_start_time_s.map(|v| format!("{:.4}", v)).unwrap_or_else(|| "null".into()),
            self.tick_interval,
            self.last_seen_tick.unwrap_or(-1),
            self.first_create_tick.unwrap_or(-1),
            self.minimap_on_entity_calls,
            self.cnpc_trooper_on_entity_calls,
            total_field_deltas,
            total_pos,
            total_ent,
            total_team,
            self.max_slot_seen,
            occupied_slots,
        );
        let _ = self.stdout.write_all(line.as_bytes());
        let _ = self.stdout.flush();
    }
}

impl Visitor for &mut ProbeVisitor {
    type Error = ProbeError;

    async fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), ProbeError> {
        let serializer_hash = entity.serializer().serializer_name.hash;

        if serializer_hash == DEADLOCK_GAMERULES_ENTITY {
            if self.match_start_time_s.is_none() {
                if let Ok(t) = entity.try_get_value::<f32>(&GAME_START_KEY) {
                    if t > 0.001 {
                        self.match_start_time_s = Some(t);
                        self.tick_interval = ctx.tick_interval();
                        self.match_started = true;
                        eprintln!(
                            "[setup] match_start_time_s={t:.4} tick_interval={:.6} tps={:.1}",
                            self.tick_interval,
                            1.0 / self.tick_interval
                        );
                    }
                }
            }
        }

        if serializer_hash == CCITADEL_TROOPER_MINIMAP {
            self.minimap_on_entity_calls += 1;
            if matches!(delta_header, DeltaHeader::CREATE) {
                self.first_create_tick = Some(ctx.tick());
                eprintln!(
                    "[CCitadelTrooperMinimap] CREATE at tick={} (matchtime_s={:.3})",
                    ctx.tick(),
                    self.match_time_s(ctx.tick())
                );
            }
            self.scan_minimap_entity(entity);
        } else if serializer_hash == CNPC_TROOPER {
            self.cnpc_trooper_on_entity_calls += 1;
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, ctx: &Context) -> Result<(), ProbeError> {
        let tick = ctx.tick();
        if tick < 0 || !self.match_started {
            return Ok(());
        }
        self.last_seen_tick = Some(tick);
        if tick % 60 != 0 {
            return Ok(());
        }
        if self.last_sample_tick == Some(tick) {
            return Ok(());
        }
        self.last_sample_tick = Some(tick);
        self.emit_sample(ctx);
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

    async fn on_packet(
        &mut self,
        _ctx: &Context,
        _packet_type: u32,
        _data: &[u8],
    ) -> Result<(), ProbeError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .expect("Usage: probe_trooper_minimap <path-to.dem>");

    eprintln!("Probing: {path}");

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = ProbeVisitor::new();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    visitor.emit_summary();
    Ok(())
}
