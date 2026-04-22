// probe_all_messages.rs
//
// Purpose
// -------
// Tallies every unique packet type seen in `on_packet` across a set of Deadlock
// replay files and cross-references the results against the spec at
// `private/specs/citadel-messages-reference.md`. Produces three outputs:
//
//   1. Per-replay packet_type tallies (id, count, name)
//   2. Aggregated tally across all input replays
//   3. Spec cross-reference: every ID documented in
//      citadel-messages-reference.md annotated VERIFIED / NOT SEEN, plus any
//      IDs in the Citadel range (300..600) that fired but are not yet in the
//      spec.
//
// This is the authoritative "what messages actually fire in real replays?"
// probe. The `**Status:**` lines in `private/specs/citadel-messages-reference.md`
// and the ✅/❌ columns in `private/specs/citadel-messages-supplemental.md` are
// produced from its output.
//
// Why it lives here
// -----------------
// Same reasoning as `probe_all_entity_classes.rs`:
//   1. It is rarely run (a handful of times per quarter, when updating the
//      messages reference/supplemental specs).
//   2. We do not want `parser/src/bin/` to accumulate one-off probes.
//   3. The verification data it produces is reference material scraped to
//      guide feature work; the provenance belongs in the private submodule.
//
// A note on valveprotos-rs staleness
// ----------------------------------
// The generated `CitadelUserMessageIds` Rust enum in `deadlock-api/valveprotos-rs`
// rev `9625c0784beca10634442ef11ede5f022ab186da` only covers IDs 300-347. IDs
// 348-366 (BossDamaged, MidBossSpawned, KillStreak, ItemPurchaseNotification,
// ImportantAbilityUsed, etc.) DO arrive in replays but are NOT in the Rust
// enum -- so `CitadelUserMessageIds::try_from(signed)` returns Err for them.
// `ECitadelGameEvents` (450-466) is also not generated.
//
// The `spec_only_name()` helper below provides human-readable names for these
// IDs so the probe output stays readable without requiring a dep bump. If you
// bump valveprotos-rs and those IDs get generated, you can delete the
// corresponding arms in `spec_only_name()`.
//
// How to run
// ----------
// This file is NOT part of the parser crate's normal build. To run it:
//
//   1. Copy it into the parser crate:
//        cp private/engineering/tools/probe_all_messages.rs \
//           parser/src/bin/probe_all_messages.rs
//   2. Ensure the parser service container is running:
//        docker compose up -d dashjump-parser
//   3. Run it via cargo inside the container (use `exec`, not `run` -- see
//      `.claude/rules/parser/CLAUDE.md`):
//        docker compose exec dashjump-parser cargo run \
//          --bin probe_all_messages -- \
//          /parser/src/replays/55423930_379917638.dem \
//          /parser/src/replays/55841493_649180947.dem \
//          /parser/src/replays/68175583_527726523.dem
//   4. After you are done, DELETE the copy from `parser/src/bin/`:
//        rm parser/src/bin/probe_all_messages.rs
//
//      Do not commit the `parser/src/bin/` copy -- keep the canonical version
//      in `private/` alongside the other probes.
//
// Dependencies (already present in parser/Cargo.toml)
// - haste
// - tokio
// - anyhow
// - thiserror

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity};
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::common::{
    EBaseEntityMessages, EBaseGameEvents, EBaseUserMessages, NetMessages, SvcMessages,
};
use haste::valveprotos::deadlock::{CitadelEntityMessageIds, CitadelUserMessageIds};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

struct Probe {
    counts: HashMap<u32, u64>,
}

impl Visitor for &mut Probe {
    type Error = ProbeError;

    async fn on_packet(
        &mut self,
        _ctx: &Context,
        packet_type: u32,
        _data: &[u8],
    ) -> Result<(), ProbeError> {
        *self.counts.entry(packet_type).or_insert(0) += 1;
        Ok(())
    }

    async fn on_entity(
        &mut self,
        _ctx: &Context,
        _delta_header: DeltaHeader,
        _entity: &Entity,
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

    async fn on_tick_end(&mut self, _ctx: &Context) -> Result<(), ProbeError> {
        Ok(())
    }
}

/// Names for Citadel IDs that are in the spec but NOT in the enum version
/// currently checked out via valveprotos-rs (IDs 348+, plus ECitadelGameEvents).
fn spec_only_name(id: u32) -> Option<&'static str> {
    match id {
        348 => Some("k_EUserMsg_BossDamaged"),
        349 => Some("k_EUserMsg_MidBossSpawned"),
        350 => Some("k_EUserMsg_RejuvStatus"),
        351 => Some("k_EUserMsg_KillStreak"),
        352 => Some("k_EUserMsg_TeamMsg"),
        353 => Some("k_EUserMsg_PlayerRespawned"),
        354 => Some("k_EUserMsg_CallCheaterVote"),
        355 => Some("k_EUserMsg_MeleeHit"),
        356 => Some("k_EUserMsg_FlexSlotUnlocked"),
        357 => Some("k_EUserMsg_SeasonalKill"),
        358 => Some("k_EUserMsg_MusicQueue"),
        359 => Some("k_EUserMsg_AG2ParamTrigger"),
        360 => Some("k_EUserMsg_ItemPurchaseNotification"),
        361 => Some("k_EUserMsg_EntityPortalled"),
        362 => Some("k_EUserMsg_StreetBrawlScoring"),
        363 => Some("k_EUserMsg_HudGameAnnouncement"),
        364 => Some("k_EUserMsg_ItemDraftReaction"),
        365 => Some("k_EUserMsg_ImportantAbilityUsed"),
        366 => Some("k_EUserMsg_BannedHeroes"),
        // ECitadelGameEvents (citadel_gameevents.proto) -- also absent from the
        // generated Rust enum in this valveprotos-rs rev, but documented upstream.
        450 => Some("GE_FireBullets"),
        451 => Some("GE_PlayerAnimEvent"),
        458 => Some("GE_ParticleSystemManager"),
        459 => Some("GE_ScreenTextPretty"),
        461 => Some("GE_BulletImpact"),
        462 => Some("GE_EnableSatVolumesEvent"),
        463 => Some("GE_PlaceSatVolumeEvent"),
        464 => Some("GE_DisableSatVolumesEvent"),
        465 => Some("GE_RemoveSatVolumeEvent"),
        466 => Some("GE_RemoveBullet"),
        _ => None,
    }
}

fn lookup_name(id: u32) -> String {
    let signed = id as i32;
    if let Ok(v) = CitadelUserMessageIds::try_from(signed) {
        return v.as_str_name().to_string();
    }
    if let Ok(v) = CitadelEntityMessageIds::try_from(signed) {
        return v.as_str_name().to_string();
    }
    if let Some(name) = spec_only_name(id) {
        return format!("{} [spec-only, not in enum]", name);
    }
    if let Ok(v) = EBaseUserMessages::try_from(signed) {
        return format!("base_um:{}", v.as_str_name());
    }
    if let Ok(v) = EBaseEntityMessages::try_from(signed) {
        return format!("base_em:{}", v.as_str_name());
    }
    if let Ok(v) = EBaseGameEvents::try_from(signed) {
        return format!("base_ge:{}", v.as_str_name());
    }
    if let Ok(v) = SvcMessages::try_from(signed) {
        return format!("svc:{}", v.as_str_name());
    }
    if let Ok(v) = NetMessages::try_from(signed) {
        return format!("net:{}", v.as_str_name());
    }
    format!("unknown({})", id)
}

fn print_tally(counts: &HashMap<u32, u64>) {
    let mut vs: Vec<(u32, u64)> = counts.iter().map(|(&k, &v)| (k, v)).collect();
    vs.sort_by_key(|&(k, _)| k);
    println!("  {:>5}  {:>10}  {}", "id", "count", "name");
    for (id, count) in vs {
        println!("  {:>5}  {:>10}  {}", id, count, lookup_name(id));
    }
}

/// Every ID listed in private/specs/citadel-messages-reference.md.
const SPEC_IDS: &[u32] = &[
    300, 303, 304, 306, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321,
    322, 323, 324, 325, 326, 327, 329, 330, 331, 332, 333, 334, 336, 337, 338, 339, 340, 341,
    342, 343, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353, 354, 355, 356, 357, 358, 359,
    360, 361, 362, 363, 364, 365, 366, 500,
];

#[tokio::main]
async fn main() -> Result<()> {
    let paths: Vec<String> = env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!(
            "Usage: probe_all_messages <replay.dem> [<replay.dem> ...]"
        );
    }

    let mut global: HashMap<u32, u64> = HashMap::new();
    let mut per_replay: Vec<(String, HashMap<u32, u64>)> = Vec::new();

    for path in &paths {
        eprintln!("Parsing: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let demo = DemoFile::start_reading(reader)?;

        let mut probe = Probe { counts: HashMap::new() };
        let mut parser = Parser::from_stream_with_visitor(demo, &mut probe)?;
        parser.run_to_end().await?;

        for (&k, &v) in &probe.counts {
            *global.entry(k).or_insert(0) += v;
        }
        per_replay.push((path.clone(), probe.counts));
    }

    println!("\n========== PER-REPLAY TALLIES ==========");
    for (path, counts) in &per_replay {
        println!("\n-- {} --", path);
        print_tally(counts);
    }

    println!("\n========== AGGREGATED ACROSS ALL REPLAYS ==========");
    print_tally(&global);

    println!("\n========== CITADEL SPEC CROSS-REFERENCE ==========");
    println!("(every ID listed in private/specs/citadel-messages-reference.md)");
    println!("  {:>5}  {:>10}  {:<9}  {}", "id", "count", "status", "name");
    let mut verified = 0u32;
    let mut missing = 0u32;
    for &id in SPEC_IDS {
        let count = global.get(&id).copied().unwrap_or(0);
        let status = if count > 0 {
            verified += 1;
            "VERIFIED"
        } else {
            missing += 1;
            "NOT SEEN"
        };
        println!(
            "  {:>5}  {:>10}  {:<9}  {}",
            id,
            count,
            status,
            lookup_name(id)
        );
    }
    println!();
    println!(
        "Spec summary: {} / {} messages VERIFIED in replays; {} NOT SEEN.",
        verified,
        SPEC_IDS.len(),
        missing
    );

    println!("\n========== CITADEL IDs SEEN BUT NOT IN SPEC ==========");
    let mut seen_extra: Vec<(u32, u64)> = global
        .iter()
        .filter(|&(&id, _)| (300..600).contains(&id))
        .filter(|&(id, _)| !SPEC_IDS.contains(id))
        .map(|(&k, &v)| (k, v))
        .collect();
    seen_extra.sort_by_key(|&(k, _)| k);
    if seen_extra.is_empty() {
        println!("  (none)");
    } else {
        for (id, count) in seen_extra {
            println!("  {:>5}  {:>10}  {}", id, count, lookup_name(id));
        }
    }

    Ok(())
}
