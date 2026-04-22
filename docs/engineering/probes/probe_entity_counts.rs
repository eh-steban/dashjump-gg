// probe_entity_counts.rs
//
// Purpose
// -------
// Runtime census of entity classes instantiated in a Deadlock replay.
//
// Canonical location: private/engineering/tools/probe_entity_counts.rs
// This is proprietary reference tooling kept in the private submodule alongside
// `probe_all_entity_classes.rs` and `probe_entity_classes.rs`. NOT part of the
// parser crate's normal build -- copy to parser/src/bin/ to run, then delete.
//
// For every entity class that actually appears at runtime, count:
//   - create_count        = number of CREATE delta events observed
//   - unique_indices      = number of distinct entity indices observed
//   - total_observations  = total on_entity callbacks (CREATE + UPDATE + PRESERVE)
//
// The static `probe_all_entity_classes` lists every class registered in the
// SendTables (a bounded universe of ~860 classes). This probe reports the
// subset that actually instantiates, plus how many instances and touches.
//
// Approach
// --------
// 1. First pass: walk the demo until CDemoSendTables, decode via prost, build
//    a HashMap<u64, String> from `fxhash::hash_bytes(class_name)` to class
//    name. Same technique as `probe_all_entity_classes` for the decode, then
//    rehashing each serializer name with the same `fxhash::hash_bytes` that
//    haste's runtime `Symbol` uses. This gives us class-name lookup by
//    `entity.serializer().serializer_name.hash` at runtime -- the
//    deadlock-api/haste fork strips `preserve-metadata` so the runtime API
//    only exposes the u64 hash.
// 2. Second pass: open the demo again and run the haste Parser with a visitor
//    that tallies on_entity callbacks keyed by `entity.serializer().serializer_name.hash`.
// 3. Print results sorted by create_count descending.
//
// How to run
// ----------
//   docker compose exec dashjump-parser cargo run --bin probe_entity_counts -- \
//       /parser/src/replays/68175583_527726523.dem
//
// Delete from parser/src/bin/ after running -- this is reference tooling that
// lives in private/engineering/tools/.
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};

use anyhow::{Context as _, Result};
use haste::demofile::DemoFile;
use haste::demostream::{CmdHeader, DemoStream};
use haste::entities::{DeltaHeader, Entity};
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::common::{CDemoSendTables, CsvcMsgFlattenedSerializer, EDemoCommands};
use prost::Message;

fn read_uvarint<R: Read>(r: &mut R) -> Result<u64> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        let mut byte = [0u8; 1];
        r.read_exact(&mut byte)?;
        let b = byte[0];
        result |= ((b & 0x7f) as u64) << shift;
        if (b & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            anyhow::bail!("varint too long");
        }
    }
}

fn build_hash_to_name_map(path: &str) -> Result<HashMap<u64, String>> {
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let mut demo_file = DemoFile::start_reading(buf_reader)?;

    let send_tables = loop {
        let cmd_header = demo_file.read_cmd_header()?;
        if cmd_header.cmd == EDemoCommands::DemSendTables {
            let cmd_body = demo_file.read_cmd(&cmd_header)?;
            break CDemoSendTables::decode(cmd_body)?;
        } else {
            demo_file.skip_cmd(&cmd_header)?;
        }
    };

    let raw = send_tables.data.unwrap_or_default();
    let mut data = &raw[..];
    let _size = read_uvarint(&mut data)?;
    let flattened = CsvcMsgFlattenedSerializer::decode(data)?;

    let symbols = &flattened.symbols;
    let mut map = HashMap::new();
    for fs in &flattened.serializers {
        if let Some(idx) = fs.serializer_name_sym {
            if let Some(name) = symbols.get(idx as usize) {
                let hash = fxhash::hash_bytes(name.as_bytes());
                map.insert(hash, name.clone());
            }
        }
    }
    Ok(map)
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ParseError(#[from] anyhow::Error);

#[derive(Default)]
struct ClassStats {
    create_count: u64,
    update_count: u64,
    delete_count: u64,
    total_observations: u64,
    unique_indices: HashSet<i32>,
}

struct CountVisitor {
    stats: HashMap<u64, ClassStats>,
}

impl CountVisitor {
    fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }
}

impl Visitor for &mut CountVisitor {
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
        _ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), ParseError> {
        let hash = entity.serializer().serializer_name.hash;
        let stats = self.stats.entry(hash).or_default();
        stats.total_observations += 1;
        stats.unique_indices.insert(entity.index());
        match delta_header {
            DeltaHeader::CREATE => stats.create_count += 1,
            DeltaHeader::UPDATE => stats.update_count += 1,
            DeltaHeader::DELETE => stats.delete_count += 1,
            _ => {}
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("Usage: probe_entity_counts <path-to.dem>")?;
    eprintln!("Probing: {path}");

    // Pass 1: build hash -> class name map from SendTables
    let hash_to_name = build_hash_to_name_map(&path)?;
    eprintln!("SendTables map built: {} serializer classes", hash_to_name.len());

    // Pass 2: run parser with counting visitor
    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut visitor = CountVisitor::new();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    eprintln!("Observed {} distinct entity class hashes at runtime", visitor.stats.len());
    eprintln!();

    // Assemble and sort results by create_count desc, then unique_indices desc.
    let mut rows: Vec<(String, &ClassStats)> = visitor
        .stats
        .iter()
        .map(|(hash, stats)| {
            let name = hash_to_name
                .get(hash)
                .cloned()
                .unwrap_or_else(|| format!("<unknown hash {hash:016x}>"));
            (name, stats)
        })
        .collect();
    rows.sort_by(|a, b| {
        b.1.create_count
            .cmp(&a.1.create_count)
            .then_with(|| b.1.unique_indices.len().cmp(&a.1.unique_indices.len()))
            .then_with(|| a.0.cmp(&b.0))
    });

    println!(
        "{:<55}  {:>8}  {:>10}  {:>10}  {:>10}  {:>12}",
        "Class", "Creates", "Unique_idx", "Updates", "Deletes", "Total_obs"
    );
    println!("{}", "-".repeat(115));
    for (name, stats) in &rows {
        println!(
            "{:<55}  {:>8}  {:>10}  {:>10}  {:>10}  {:>12}",
            name,
            stats.create_count,
            stats.unique_indices.len(),
            stats.update_count,
            stats.delete_count,
            stats.total_observations
        );
    }

    println!();
    println!("Total runtime classes: {}", rows.len());
    let total_creates: u64 = rows.iter().map(|r| r.1.create_count).sum();
    let total_unique: usize = rows.iter().map(|r| r.1.unique_indices.len()).sum();
    println!("Total CREATE events across all classes: {total_creates}");
    println!("Total unique entity indices across all classes: {total_unique}");

    // Report the delta: classes defined in SendTables but never observed at runtime.
    let observed_hashes: HashSet<u64> = visitor.stats.keys().copied().collect();
    let never_observed: Vec<String> = hash_to_name
        .iter()
        .filter(|(h, _)| !observed_hashes.contains(h))
        .map(|(_, n)| n.clone())
        .collect();
    eprintln!("Classes in SendTables but never observed at runtime: {}", never_observed.len());

    Ok(())
}
