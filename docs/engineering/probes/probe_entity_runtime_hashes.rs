// probe_entity_runtime_hashes.rs
//
// Spike: boss-serializer-hash-drift (2026-04-14) -- step 4 runtime comparison probe.
//
// Purpose
// -------
// For every CREATE event, records the runtime serializer hash (what haste stores in
// `entity.serializer().serializer_name.hash`) plus the first entity index and tick
// at which each distinct hash was seen. Joined with the output of
// `probe_all_entity_classes` (extended to emit `fxhash::hash_bytes(symbol)` alongside
// each symbol name) to answer spike question (a): does runtime hash equal
// `fxhash::hash_bytes(symbol_bytes)`.
//
// The original spike run (2026-04-14) confirmed 190/190 and 177/177 runtime hashes
// matched a symbol in the static table for replays 55423930 and 68175583 respectively,
// establishing that the runtime hash is just the fxhash of the class-name string. The
// probe is retained here so future investigations into hash-based entity identification
// (e.g. when adding new `_ENTITY` constants in `parser/src/entities/constants.rs`) can
// re-verify the assumption without re-deriving it.
//
// How to run
// ----------
// Copy-to-bin-then-delete workflow (see probe_all_entity_classes.rs for rationale):
//
//   1. cp private/engineering/tools/probe_entity_runtime_hashes.rs \
//         parser/src/bin/probe_entity_runtime_hashes.rs
//   2. docker compose exec dashjump-parser cargo run \
//         --bin probe_entity_runtime_hashes -- \
//         /parser/src/replays/<match_id>_<replay_salt>.dem
//   3. rm parser/src/bin/probe_entity_runtime_hashes.rs
//
// Do not commit the `parser/src/bin/` copy -- it must stay in `private/` so the
// scraped-data provenance stays inside the private submodule.
//
// Output format (stdout)
// ----------------------
//   # runtime_hash\tentity_index_first_seen\ttick_first_seen
//   <u64>\t<i32>\t<i32>
//
// One row per distinct `serializer_name.hash` seen during the full parse. Tick is
// a raw demo tick (60 ticks/sec); convert to seconds by subtracting match-start tick
// if you need match-relative time.
//
// Dependencies (already present in parser/Cargo.toml)
// ---------------------------------------------------
// - haste (git = "https://github.com/deadlock-api/haste.git")
// - tokio (features = ["full"])
// - anyhow, thiserror

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::{Context as AnyhowCtx, Result};
use haste::demofile::DemoFile;
use haste::entities::{DeltaHeader, Entity};
use haste::parser::{Context, Parser, Visitor};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

#[derive(Default)]
struct RuntimeHashVisitor {
    first_seen: HashMap<u64, (i32, i32)>, // hash -> (entity_index, tick)
}

impl Visitor for &mut RuntimeHashVisitor {
    type Error = ProbeError;

    async fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), Self::Error> {
        if !matches!(delta_header, DeltaHeader::CREATE) {
            return Ok(());
        }
        let hash = entity.serializer().serializer_name.hash;
        if self.first_seen.contains_key(&hash) {
            return Ok(());
        }
        self.first_seen
            .insert(hash, (entity.index() as i32, ctx.tick()));
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("Usage: probe_entity_runtime_hashes <path-to.dem>")?;
    eprintln!("Probing: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = RuntimeHashVisitor::default();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    let mut rows: Vec<(u64, i32, i32)> = visitor
        .first_seen
        .iter()
        .map(|(h, (idx, tick))| (*h, *idx, *tick))
        .collect();
    rows.sort_by_key(|r| r.0);

    println!("# runtime_hash\tentity_index_first_seen\ttick_first_seen");
    for (h, idx, tick) in rows {
        println!("{h}\t{idx}\t{tick}");
    }

    Ok(())
}
