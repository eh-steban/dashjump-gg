// probe_all_entity_classes.rs
//
// Purpose
// -------
// Enumerates every entity class name (and type identifier) present in a Deadlock demo's
// SendTables, plus optional per-class field dumps. This is the authoritative "what entity
// classes / fields exist in a replay?" probe -- the data it produces underpins the
// [probe-classes] citations in `private/specs/entity-types-reference.md` and
// `private/specs/entity-fields-reference.md`.
//
// This is proprietary reference tooling. It lives in `private/` on purpose:
//   1. It's rarely run (a handful of times per quarter, when updating the entity specs).
//   2. We don't want `parser/src/bin/` to accumulate one-off scripts.
//   3. The extracted class/field enumerations are the result of manual scraping and
//      reverse engineering effort we want to keep in the private submodule.
//
// Why we can't just call haste's entity API
// -----------------------------------------
// The `deadlock-api/haste` fork strips the `preserve-metadata` feature, so `Symbol`
// only stores a hash (`u64`); we cannot recover class names via the haste entity API.
// `CDemoSendTables` still carries raw ASCII symbols in the proto `symbols` field;
// decoding the proto directly gives us the strings.
//
// How to run
// ----------
// This file is NOT part of the parser crate's normal build. To run it:
//
//   1. Copy it into the parser crate:
//        cp private/engineering/tools/probe_all_entity_classes.rs \
//           parser/src/bin/probe_all_entity_classes.rs
//   2. Ensure the parser service container is running:
//        docker compose up -d dashjump-parser
//   3. Run it via cargo inside the container (the `exec` pattern -- see
//      `.claude/rules/parser/CLAUDE.md` for why we use exec, not run):
//        docker compose exec dashjump-parser cargo run \
//          --bin probe_all_entity_classes -- \
//          /parser/src/replays/<match_id>_<replay_salt>.dem [class_filter_substr]
//
//      Example (dump all CNPC_* class fields):
//        docker compose exec dashjump-parser cargo run \
//          --bin probe_all_entity_classes -- \
//          /parser/src/replays/68175583_527726523.dem CNPC_
//   4. After you're done, DELETE the copy from `parser/src/bin/`:
//        rm parser/src/bin/probe_all_entity_classes.rs
//
//      Do not commit the `parser/src/bin/` copy -- it must stay in `private/` so the
//      scraped-data provenance and reverse-engineering notes stay inside the private
//      submodule.
//
// Dependencies (already present in parser/Cargo.toml)
// ---------------------------------------------------
// - haste (git = "https://github.com/deadlock-api/haste.git")
// - prost
// - anyhow
//
// Reference: crates/haste_core/src/flattenedserializers.rs FlattenedSerializers::parse

use std::collections::BTreeSet;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};

use anyhow::{Context as _, Result};
use haste::demofile::DemoFile;
use haste::demostream::DemoStream;
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

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("Usage: probe_all_entity_classes <path-to.dem> [class_filter_substr]")?;
    let filter: Option<String> = env::args().nth(2);
    eprintln!("Probing: {}", path);
    if let Some(f) = &filter {
        eprintln!("Field dump filter: {f}");
    }

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let mut demo_file = DemoFile::start_reading(buf_reader)?;

    // Walk commands until we find the SendTables packet. It appears in the pre-match
    // tick range (tick <= 0) near the start of the demo.
    let send_tables = loop {
        let cmd_header = demo_file.read_cmd_header()?;
        if cmd_header.cmd == EDemoCommands::DemSendTables {
            let cmd_body = demo_file.read_cmd(&cmd_header)?;
            break CDemoSendTables::decode(cmd_body)?;
        } else {
            demo_file.skip_cmd(&cmd_header)?;
        }
    };

    // The data blob starts with a uvarint size prefix, then the actual
    // CsvcMsgFlattenedSerializer message. Strip the prefix before decoding.
    let raw = send_tables.data.unwrap_or_default();
    let mut data = &raw[..];
    let _size = read_uvarint(&mut data)?;
    let flattened = CsvcMsgFlattenedSerializer::decode(data)?;

    let symbols = &flattened.symbols;
    eprintln!("symbol table size: {}", symbols.len());

    // (1) Serializer class names -- these are the entity class names.
    let mut serializer_names: BTreeSet<String> = BTreeSet::new();
    for fs in &flattened.serializers {
        if let Some(idx) = fs.serializer_name_sym {
            if let Some(name) = symbols.get(idx as usize) {
                serializer_names.insert(name.clone());
            }
        }
    }

    println!("==========================================");
    println!("SERIALIZER CLASS NAMES (entity classes)");
    println!("==========================================");
    println!("count: {}", serializer_names.len());
    println!();
    for n in &serializer_names {
        println!("{n}");
    }

    // (2) Everything else in the symbol table that looks like a C-class identifier.
    // Catches classes that appear only as field types (e.g. CHandle<CCitadel_Powerup>).
    println!();
    println!("==========================================");
    println!("C-PREFIXED SYMBOLS (types referenced by fields)");
    println!("==========================================");
    let mut c_symbols: BTreeSet<String> = BTreeSet::new();
    for sym in symbols {
        // Split on non-identifier chars so we pick up C-names inside things like
        // "CHandle< CCitadel_Powerup >" or "CUtlVector< CHandle< CCitadelPlayerPawn > >".
        let tokens: Vec<&str> = sym
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .collect();
        for token in tokens {
            let mut chars = token.chars();
            let first = chars.next();
            let second = chars.next();
            if first == Some('C')
                && token.len() > 2
                && matches!(second, Some(c) if c.is_ascii_uppercase())
            {
                c_symbols.insert(token.to_string());
            }
        }
    }
    // Exclude classes already listed as serializers
    let extra: Vec<&String> = c_symbols.difference(&serializer_names).collect();
    println!("count (excluding serializers): {}", extra.len());
    println!();
    for n in extra {
        println!("{n}");
    }

    // (3) Optional per-class field dump when a filter substring is supplied.
    if let Some(filter) = filter {
        println!();
        println!("==========================================");
        println!("FIELD DUMPS FOR CLASSES MATCHING '{filter}'");
        println!("==========================================");
        for fs in &flattened.serializers {
            let name = match fs.serializer_name_sym {
                Some(idx) => symbols.get(idx as usize).cloned().unwrap_or_default(),
                None => continue,
            };
            if !name.contains(&filter) {
                continue;
            }
            println!();
            println!("### {name}  ({} fields)", fs.fields_index.len());
            for field_idx in &fs.fields_index {
                let field = match flattened.fields.get(*field_idx as usize) {
                    Some(f) => f,
                    None => continue,
                };
                let var_name = field
                    .var_name_sym
                    .and_then(|i| symbols.get(i as usize).cloned())
                    .unwrap_or_else(|| "<unnamed>".to_string());
                let var_type = field
                    .var_type_sym
                    .and_then(|i| symbols.get(i as usize).cloned())
                    .unwrap_or_else(|| "<untyped>".to_string());
                println!("  {var_name}: {var_type}");
            }
        }
    }

    Ok(())
}
