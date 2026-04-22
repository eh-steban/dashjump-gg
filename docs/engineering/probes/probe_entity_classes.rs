// probe_entity_classes.rs
//
// Purpose
// -------
// Runtime probe that resolves a hand-picked set of entity indices (WATCHED_INDICES)
// back to their serializer class names, recording first-observation tick, match
// second, delta kind (CREATE/UPDATE/etc.), and initial m_iHealth. Designed to answer
// "what class does entity N belong to?" when investigating unexpected entries in
// `boss_tracker` health_timeline output or other index-driven debugging.
//
// Why it's in private/
// --------------------
// Proprietary reference tooling -- kept in the private submodule so we don't lose it
// and don't accumulate one-off scripts in `parser/src/bin/`. Companion to
// `probe_all_entity_classes.rs`, which solves the static "what classes exist?"
// question; this one solves the runtime "what is entity #73?" question.
//
// !!! CURRENTLY BROKEN AGAINST deadlock-api/haste !!!
// ---------------------------------------------------
// This file was last known to work against `blukai/haste` with the `preserve-metadata`
// feature enabled. The `deadlock-api/haste` fork we migrated to strips
// `preserve-metadata`, so `Symbol` only stores a `u64` hash -- there is no
// `serializer_name.str` field to read from `entity.serializer()` at runtime. The
// `.str` accesses on lines referencing `entity.serializer().serializer_name.str` will
// fail to compile.
//
// To port before running:
//   1. Parse CDemoSendTables once up front (see `probe_all_entity_classes.rs`) to
//      build a `HashMap<u64, String>` from `fxhash::hash_bytes(class_name)` to
//      the class name string.
//   2. At runtime, compute the class hash from the entity serializer's stored hash
//      (whatever accessor the current haste API exposes) and look it up in the map.
//   3. Replace all `entity.serializer().serializer_name.str` references with that
//      lookup.
//
// How to run (after porting)
// --------------------------
//   1. Copy it into the parser crate:
//        cp private/engineering/tools/probe_entity_classes.rs \
//           parser/src/bin/probe_entity_classes.rs
//   2. Edit WATCHED_INDICES to the entity indices you want to resolve.
//   3. Ensure the parser service container is running:
//        docker compose up -d dashjump-parser
//   4. Run it via cargo inside the container (exec, not run -- see
//      `.claude/rules/parser/CLAUDE.md`):
//        docker compose exec dashjump-parser cargo run \
//          --bin probe_entity_classes -- \
//          /parser/src/replays/<match_id>_<replay_salt>.dem
//   5. After you're done, DELETE the copy from `parser/src/bin/` -- do not commit
//      the `parser/src/bin/` copy.
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity, fkey_from_path};
use haste::parser::{Context, Parser, Visitor};

// Entity indices to investigate. Covers all four groups from the question.
const WATCHED_INDICES: &[i32] = &[
    // Group 1: mid-match health=0 cluster
    73, 74, 75, 76, 77,
    // Group 2: early health=0 cluster
    2917, 2919, 2940,
    // Group 3: suspected Guardians
    2527, 2528, 2529, 2530, 2531, 2532,
    // Group 4: suspected Patrons
    294, 295,
];

const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);
const HEALTH_KEY: u64 = fkey_from_path(&["m_iHealth"]);
const GAMERULES_HASH: &[u8] = b"CCitadelGameRulesProxy";

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ParseError(#[from] anyhow::Error);

struct EntityRecord {
    class_name: String,
    first_delta: String,
    tick: u32,
    match_second: i32,
    health: Option<i32>,
}

struct ProbeVisitor {
    watched: HashSet<i32>,
    game_start_time: Option<f32>,
    // Records indexed by entity index -- only first observation stored
    records: HashMap<i32, EntityRecord>,
}

impl ProbeVisitor {
    fn new() -> Self {
        Self {
            watched: WATCHED_INDICES.iter().copied().collect(),
            game_start_time: None,
            records: HashMap::new(),
        }
    }

    fn match_second(&self, ctx: &Context) -> i32 {
        match self.game_start_time {
            Some(start) => {
                let current_time = ctx.tick() as f32 * ctx.tick_interval();
                (current_time - start) as i32
            }
            None => -1,
        }
    }
}

impl Visitor for &mut ProbeVisitor {
    type Error = ParseError;

    async fn on_cmd(&mut self, _ctx: &Context, _ch: &CmdHeader, _d: &[u8]) -> Result<(), ParseError> {
        Ok(())
    }

    async fn on_packet(&mut self, _ctx: &Context, _t: u32, _data: &[u8]) -> Result<(), ParseError> {
        Ok(())
    }

    async fn on_tick_end(&mut self, _ctx: &Context) -> Result<(), ParseError> {
        Ok(())
    }

    async fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), ParseError> {
        // Extract game start time from game rules proxy
        let class_name_bytes = entity.serializer().serializer_name.str.as_bytes();
        if class_name_bytes == GAMERULES_HASH {
            if let Some(t) = entity.get_value::<f32>(&GAME_START_KEY) {
                if t > 0.0 && self.game_start_time.is_none() {
                    self.game_start_time = Some(t);
                }
            }
        }

        let idx = entity.index();
        if !self.watched.contains(&idx) {
            return Ok(());
        }

        // Record only the first observation per entity index (could be CREATE or UPDATE
        // if the entity existed before the first full packet snapshot)
        if self.records.contains_key(&idx) {
            return Ok(());
        }

        let class_name = entity
            .serializer()
            .serializer_name
            .str
            .to_string();

        let delta_str = match delta_header {
            DeltaHeader::CREATE => "CREATE",
            DeltaHeader::UPDATE => "UPDATE",
            DeltaHeader::DELETE => "DELETE",
            DeltaHeader::PRESERVE => "PRESERVE",
        }
        .to_string();

        let health: Option<i32> = entity.get_value(&HEALTH_KEY);
        let ms = self.match_second(ctx);

        self.records.insert(
            idx,
            EntityRecord {
                class_name,
                first_delta: delta_str,
                tick: ctx.tick(),
                match_second: ms,
                health,
            },
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args().nth(1).expect("Usage: probe_entity_classes <path-to.dem>");
    println!("Probing: {}", path);
    println!();

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = ProbeVisitor::new();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    // Print results sorted by entity index
    let mut indices: Vec<i32> = visitor.records.keys().copied().collect();
    indices.sort();

    println!(
        "{:<8}  {:<45}  {:<8}  {:>5}  {:>7}  {:>10}",
        "Index", "Class", "Delta", "Tick", "Match_s", "Health"
    );
    println!("{}", "-".repeat(95));

    for idx in &indices {
        let r = &visitor.records[idx];
        let health_str = match r.health {
            Some(h) => h.to_string(),
            None => "None".to_string(),
        };
        println!(
            "{:<8}  {:<45}  {:<8}  {:>5}  {:>7}  {:>10}",
            idx, r.class_name, r.first_delta, r.tick, r.match_second, health_str
        );
    }

    // Report any watched indices that never appeared
    println!();
    let missing: Vec<i32> = WATCHED_INDICES
        .iter()
        .filter(|i| !visitor.records.contains_key(i))
        .copied()
        .collect();
    if missing.is_empty() {
        println!("All watched indices observed.");
    } else {
        println!("Never observed (not present in demo): {:?}", missing);
    }

    Ok(())
}
