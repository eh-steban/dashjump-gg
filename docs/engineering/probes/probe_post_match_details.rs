// probe_post_match_details.rs
//
// Purpose
// -------
// Validates the CMsgMatchMetaDataContents schema against real replays by decoding
// CCitadelUserMsg_PostMatchDetails (ID 316) and reporting field presence, counts,
// and sample values for all major sub-messages.
//
// Answers open questions from private/specs/citadel-gcmessages-common-reference.md
// Section 6:
//   #1: assigned_lane encoding (0-indexed sequential vs. CMsgLaneColor values)
//   #3: coordinate decode formula for CMsgMatchPlayerPathsData
//   #4: PlayerStats snapshot interval (time_stamp_s cadence)
//   #5: mid_boss team_killed vs. team_claimed divergence
//
// Type name resolution (Task 1)
// -----------------------------
// The generated Rust struct name is CMsgMatchMetaDataContentsPatched (no trailing l).
// The spec doc cites three name variants:
//   - patch.proto: CMsgMatchMetaDataContentsPatchedl (trailing l)
//   - inline copy in gcmessages_common: concatenated/malformed
//   - haste migration docs: CMsgMatchMetaDataContentsPatched
// The valveprotos-rs generated code (deadlock-api/haste fork) uses:
//   pub struct CMsgMatchMetaDataContentsPatched
// This is confirmed in /tmp/parser-target/debug/build/valveprotos-*/out/deadlock.rs.
//
// Decode path
// -----------
// The outer CCitadelUserMsg_PostMatchDetails contains a single `match_details: bytes`
// field that decodes directly as CMsgMatchMetaDataContentsPatched. The CMsgMatchMetaData
// intermediate envelope is NOT present -- the bytes are the patched contents directly.
// This is confirmed by parse_local.rs which decodes in one step without CMsgMatchMetaData.
//
// How to run
// ----------
// This file is NOT part of the parser crate's normal build. To run it:
//
//   1. Copy it into the parser crate:
//        cp private/engineering/tools/probe_post_match_details.rs \
//           parser/src/bin/probe_post_match_details.rs
//   2. Ensure the parser service container is running:
//        docker compose up -d dashjump-parser
//   3. Run it via cargo inside the container (use `exec`, not `run` -- see
//      .claude/rules/parser/CLAUDE.md):
//        docker compose exec dashjump-parser cargo run \
//          --bin probe_post_match_details -- \
//          /parser/src/replays/55423930_379917638.dem \
//          /parser/src/replays/55841493_649180947.dem \
//          /parser/src/replays/68175583_527726523.dem
//   4. After you are done, DELETE the copy from parser/src/bin/:
//        rm parser/src/bin/probe_post_match_details.rs
//
//      Do not commit the parser/src/bin/ copy -- keep the canonical version
//      in private/ alongside the other probes.
//
// Dependencies (already present in parser/Cargo.toml)
// - haste
// - tokio
// - anyhow
// - thiserror
// - prost

use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::{Context, Result};
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity};
use haste::parser::{Context as HasteContext, Parser, Visitor};
use haste::valveprotos::deadlock::{
    CCitadelUserMsgPostMatchDetails, CitadelUserMessageIds, CMsgMatchMetaDataContentsPatched,
};
use prost::Message;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

impl From<prost::DecodeError> for ProbeError {
    fn from(e: prost::DecodeError) -> Self {
        ProbeError(anyhow::Error::from(e))
    }
}

/// Per-replay result collected during the parse pass.
struct ReplayResult {
    replay_name: String,
    // MatchInfo scalars
    duration_s: Option<u32>,
    match_outcome: Option<i32>,
    winning_team: Option<i32>,
    start_time: Option<u32>,
    match_id: Option<u64>,
    legacy_objectives_mask: Option<u32>,
    game_mode: Option<i32>,
    match_mode: Option<i32>,
    objectives_mask_team0: Option<u64>,
    objectives_mask_team1: Option<u64>,
    is_high_skill_range_parties: Option<bool>,
    low_pri_pool: Option<bool>,
    new_player_pool: Option<bool>,
    average_badge_team0: Option<u32>,
    average_badge_team1: Option<u32>,
    game_mode_version: Option<u32>,
    rewards_eligible: Option<bool>,
    not_scored: Option<bool>,
    // MatchInfo repeated counts
    player_count: usize,
    objective_count: usize,
    pause_count: usize,
    mid_boss_count: usize,
    custom_user_stat_count: usize,
    watched_death_replay_count: usize,
    // Players[] -- per-player field summaries
    player_summaries: Vec<PlayerSummary>,
    // PlayerStats[] -- snapshot intervals observed across all players
    all_timestamp_intervals: Vec<u32>,
    stats_per_player: Vec<usize>,
    // CMsgMatchPlayerPathsData
    paths_interval_s: Option<f32>,
    paths_x_resolution: Option<u32>,
    paths_y_resolution: Option<u32>,
    paths_player_count: usize,
    // Per-player path bounding boxes and sample counts
    path_summaries: Vec<PathSummary>,
    // CMsgMatchPlayerDamageMatrix
    damage_sample_times: Vec<u32>,
    damage_dealer_count: usize,
    damage_source_names: Vec<String>,
    // mid_boss steal validation
    mid_boss_records: Vec<MidBossRecord>,
    // Coordinate decode validation for first path player
    first_path_coord_sample: Option<CoordSample>,
}

struct PlayerSummary {
    player_slot: Option<u32>,
    account_id: Option<u32>,
    hero_id: Option<u32>,
    team: Option<i32>,
    kills: Option<u32>,
    deaths: Option<u32>,
    assists: Option<u32>,
    net_worth: Option<u32>,
    last_hits: Option<u32>,
    denies: Option<u32>,
    ability_points: Option<u32>,
    assigned_lane: Option<u32>,
    level: Option<u32>,
    party: Option<u32>,
    abandon_match_time_s: Option<u32>,
    rewards_eligible: Option<bool>,
    death_count: usize,
    item_count: usize,
    stats_count: usize,
    ping_count: usize,
    ability_stat_count: usize,
    stats_type_stat_count: usize,
    book_reward_count: usize,
}

struct PathSummary {
    player_slot: u32,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    sample_count: usize,
    health_count: usize,
    combat_type_count: usize,
    move_type_count: usize,
}

struct MidBossRecord {
    team_killed: Option<i32>,
    team_claimed: Option<i32>,
    destroyed_time_s: Option<u32>,
    is_steal: bool,
}

struct CoordSample {
    player_slot: u32,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    x_resolution: u32,
    y_resolution: u32,
    x_raw: u32,
    y_raw: u32,
    x_world: f32,
    y_world: f32,
}

struct Probe {
    result: Option<ReplayResult>,
    replay_name: String,
}

impl Probe {
    fn new(replay_name: String) -> Self {
        Self {
            result: None,
            replay_name,
        }
    }
}

impl Visitor for &mut Probe {
    type Error = ProbeError;

    async fn on_packet(
        &mut self,
        _ctx: &HasteContext,
        packet_type: u32,
        data: &[u8],
    ) -> Result<(), ProbeError> {
        if packet_type != CitadelUserMessageIds::KEUserMsgPostMatchDetails as u32 {
            return Ok(());
        }

        let outer = CCitadelUserMsgPostMatchDetails::decode(data)
            .context("failed to decode CCitadelUserMsgPostMatchDetails")?;

        let bytes = match outer.match_details {
            Some(b) => b,
            None => {
                eprintln!("[probe] match_details bytes missing from outer message");
                return Ok(());
            }
        };

        // Decode directly as CMsgMatchMetaDataContentsPatched -- no CMsgMatchMetaData
        // intermediate step. parse_local.rs confirms this one-step path works.
        let contents = CMsgMatchMetaDataContentsPatched::decode(bytes.as_slice())
            .context("failed to decode CMsgMatchMetaDataContentsPatched")?;

        let match_info = match contents.match_info {
            Some(mi) => mi,
            None => {
                eprintln!("[probe] match_info missing from CMsgMatchMetaDataContentsPatched");
                return Ok(());
            }
        };

        let mut result = ReplayResult {
            replay_name: self.replay_name.clone(),
            duration_s: match_info.duration_s,
            match_outcome: match_info.match_outcome,
            winning_team: match_info.winning_team,
            start_time: match_info.start_time,
            match_id: match_info.match_id,
            legacy_objectives_mask: match_info.legacy_objectives_mask,
            game_mode: match_info.game_mode,
            match_mode: match_info.match_mode,
            objectives_mask_team0: match_info.objectives_mask_team0,
            objectives_mask_team1: match_info.objectives_mask_team1,
            is_high_skill_range_parties: match_info.is_high_skill_range_parties,
            low_pri_pool: match_info.low_pri_pool,
            new_player_pool: match_info.new_player_pool,
            average_badge_team0: match_info.average_badge_team0,
            average_badge_team1: match_info.average_badge_team1,
            game_mode_version: match_info.game_mode_version,
            rewards_eligible: match_info.rewards_eligible,
            not_scored: match_info.not_scored,
            player_count: match_info.players.len(),
            objective_count: match_info.objectives.len(),
            pause_count: match_info.match_pauses.len(),
            mid_boss_count: match_info.mid_boss.len(),
            custom_user_stat_count: match_info.custom_user_stats.len(),
            watched_death_replay_count: match_info.watched_death_replays.len(),
            player_summaries: Vec::new(),
            all_timestamp_intervals: Vec::new(),
            stats_per_player: Vec::new(),
            paths_interval_s: None,
            paths_x_resolution: None,
            paths_y_resolution: None,
            paths_player_count: 0,
            path_summaries: Vec::new(),
            damage_sample_times: Vec::new(),
            damage_dealer_count: 0,
            damage_source_names: Vec::new(),
            mid_boss_records: Vec::new(),
            first_path_coord_sample: None,
        };

        // Players[]
        for p in &match_info.players {
            let mut prev_ts: Option<u32> = None;
            let mut local_intervals: Vec<u32> = Vec::new();
            for stat in &p.stats {
                if let Some(ts) = stat.time_stamp_s {
                    if let Some(prev) = prev_ts {
                        if ts > prev {
                            local_intervals.push(ts - prev);
                            result.all_timestamp_intervals.push(ts - prev);
                        }
                    }
                    prev_ts = Some(ts);
                }
            }
            result.stats_per_player.push(p.stats.len());

            result.player_summaries.push(PlayerSummary {
                player_slot: p.player_slot,
                account_id: p.account_id,
                hero_id: p.hero_id,
                team: p.team,
                kills: p.kills,
                deaths: p.deaths,
                assists: p.assists,
                net_worth: p.net_worth,
                last_hits: p.last_hits,
                denies: p.denies,
                ability_points: p.ability_points,
                assigned_lane: p.assigned_lane,
                level: p.level,
                party: p.party,
                abandon_match_time_s: p.abandon_match_time_s,
                rewards_eligible: p.rewards_eligible,
                death_count: p.death_details.len(),
                item_count: p.items.len(),
                stats_count: p.stats.len(),
                ping_count: p.pings.len(),
                ability_stat_count: p.ability_stats.len(),
                stats_type_stat_count: p.stats_type_stat.len(),
                book_reward_count: p.book_rewards.len(),
            });
        }

        // CMsgMatchPlayerPathsData
        if let Some(paths_data) = &match_info.match_paths {
            result.paths_interval_s = paths_data.interval_s;
            result.paths_x_resolution = paths_data.x_resolution;
            result.paths_y_resolution = paths_data.y_resolution;
            result.paths_player_count = paths_data.paths.len();

            let x_res = paths_data.x_resolution.unwrap_or(1) as f32;
            let y_res = paths_data.y_resolution.unwrap_or(1) as f32;

            for (pi, path) in paths_data.paths.iter().enumerate() {
                let sample_count = path.x_pos.len();
                result.path_summaries.push(PathSummary {
                    player_slot: path.player_slot.unwrap_or(0),
                    x_min: path.x_min.unwrap_or(0.0),
                    x_max: path.x_max.unwrap_or(0.0),
                    y_min: path.y_min.unwrap_or(0.0),
                    y_max: path.y_max.unwrap_or(0.0),
                    sample_count,
                    health_count: path.health.len(),
                    combat_type_count: path.combat_type.len(),
                    move_type_count: path.move_type.len(),
                });

                // Coordinate decode validation -- first path, first sample
                if pi == 0 && result.first_path_coord_sample.is_none() {
                    if let (Some(&x_raw), Some(&y_raw)) =
                        (path.x_pos.first(), path.y_pos.first())
                    {
                        let x_min = path.x_min.unwrap_or(0.0);
                        let x_max = path.x_max.unwrap_or(0.0);
                        let y_min = path.y_min.unwrap_or(0.0);
                        let y_max = path.y_max.unwrap_or(0.0);
                        // Hypothesis: x_world = x_min + (x_raw / x_resolution) * (x_max - x_min)
                        let x_world = x_min + (x_raw as f32 / x_res) * (x_max - x_min);
                        let y_world = y_min + (y_raw as f32 / y_res) * (y_max - y_min);
                        result.first_path_coord_sample = Some(CoordSample {
                            player_slot: path.player_slot.unwrap_or(0),
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            x_resolution: paths_data.x_resolution.unwrap_or(0),
                            y_resolution: paths_data.y_resolution.unwrap_or(0),
                            x_raw,
                            y_raw,
                            x_world,
                            y_world,
                        });
                    }
                }
            }
        }

        // CMsgMatchPlayerDamageMatrix
        if let Some(dm) = &match_info.damage_matrix {
            result.damage_sample_times = dm.sample_time_s.clone();
            result.damage_dealer_count = dm.damage_dealers.len();
            if let Some(sd) = &dm.source_details {
                result.damage_source_names = sd.source_name.clone();
            }
        }

        // MidBoss records
        for mb in &match_info.mid_boss {
            let killed = mb.team_killed;
            let claimed = mb.team_claimed;
            let is_steal = match (killed, claimed) {
                (Some(k), Some(c)) => k != c,
                _ => false,
            };
            result.mid_boss_records.push(MidBossRecord {
                team_killed: mb.team_killed,
                team_claimed: mb.team_claimed,
                destroyed_time_s: mb.destroyed_time_s,
                is_steal,
            });
        }

        self.result = Some(result);
        Ok(())
    }

    async fn on_entity(
        &mut self,
        _ctx: &HasteContext,
        _delta_header: DeltaHeader,
        _entity: &Entity,
    ) -> Result<(), ProbeError> {
        Ok(())
    }

    async fn on_cmd(
        &mut self,
        _ctx: &HasteContext,
        _cmd_header: &CmdHeader,
        _data: &[u8],
    ) -> Result<(), ProbeError> {
        Ok(())
    }

    async fn on_tick_end(&mut self, _ctx: &HasteContext) -> Result<(), ProbeError> {
        Ok(())
    }
}

fn print_result(r: &ReplayResult) {
    println!("=== Replay: {} ===", r.replay_name);
    println!();
    println!("--- MatchInfo scalars ---");
    println!("  duration_s:                  {:?}", r.duration_s);
    println!("  match_id:                    {:?}", r.match_id);
    println!("  start_time:                  {:?}", r.start_time);
    println!("  match_outcome:               {:?}", r.match_outcome);
    println!("  winning_team:                {:?}", r.winning_team);
    println!("  game_mode:                   {:?}", r.game_mode);
    println!("  match_mode:                  {:?}", r.match_mode);
    println!("  game_mode_version:           {:?}", r.game_mode_version);
    println!("  legacy_objectives_mask:      {:?}", r.legacy_objectives_mask);
    println!("  objectives_mask_team0:       {:?}", r.objectives_mask_team0);
    println!("  objectives_mask_team1:       {:?}", r.objectives_mask_team1);
    println!("  is_high_skill_range_parties: {:?}", r.is_high_skill_range_parties);
    println!("  low_pri_pool:                {:?}", r.low_pri_pool);
    println!("  new_player_pool:             {:?}", r.new_player_pool);
    println!("  average_badge_team0:         {:?}", r.average_badge_team0);
    println!("  average_badge_team1:         {:?}", r.average_badge_team1);
    println!("  rewards_eligible:            {:?}", r.rewards_eligible);
    println!("  not_scored:                  {:?}", r.not_scored);
    println!();
    println!("--- MatchInfo repeated counts ---");
    println!("  players:              {}", r.player_count);
    println!("  objectives:           {}", r.objective_count);
    println!("  match_pauses:         {}", r.pause_count);
    println!("  mid_boss:             {}", r.mid_boss_count);
    println!("  custom_user_stats:    {}", r.custom_user_stat_count);
    println!("  watched_death_replays:{}", r.watched_death_replay_count);
    println!();
    println!("--- Players[] (30+ fields per player) ---");
    for ps in &r.player_summaries {
        println!(
            "  slot={:?} acct={:?} hero={:?} team={:?} party={:?} lane={:?} lvl={:?}",
            ps.player_slot,
            ps.account_id,
            ps.hero_id,
            ps.team,
            ps.party,
            ps.assigned_lane,
            ps.level
        );
        println!(
            "    k={:?} d={:?} a={:?} nw={:?} lh={:?} denies={:?} ap={:?}",
            ps.kills,
            ps.deaths,
            ps.assists,
            ps.net_worth,
            ps.last_hits,
            ps.denies,
            ps.ability_points
        );
        println!(
            "    abandon_s={:?} rewards_eligible={:?}",
            ps.abandon_match_time_s, ps.rewards_eligible
        );
        println!(
            "    deaths={} items={} stats={} pings={} ability_stats={} stats_type_stat_floats={} book_rewards={}",
            ps.death_count,
            ps.item_count,
            ps.stats_count,
            ps.ping_count,
            ps.ability_stat_count,
            ps.stats_type_stat_count,
            ps.book_reward_count
        );
    }
    println!();
    println!("--- assigned_lane values ---");
    let lanes: Vec<Option<u32>> = r.player_summaries.iter().map(|p| p.assigned_lane).collect();
    println!("  {:?}", lanes);
    println!("  (CMsgLaneColor values: Yellow=1, Green=3, Blue=4, Purple=6)");
    println!("  (Sequential 1-4 or 0-3 hypothesis: check if values are in [0..6] contiguous)");
    println!();
    println!("--- PlayerStats[] snapshot intervals ---");
    println!("  stats_per_player: {:?}", r.stats_per_player);
    if !r.all_timestamp_intervals.is_empty() {
        let mut sorted = r.all_timestamp_intervals.clone();
        sorted.sort();
        sorted.dedup();
        println!("  unique interval values observed: {:?}", sorted);
        let min_ts = *r.all_timestamp_intervals.iter().min().unwrap_or(&0);
        let max_ts = *r.all_timestamp_intervals.iter().max().unwrap_or(&0);
        println!("  interval range: {}s .. {}s", min_ts, max_ts);
        let most_common = mode(&r.all_timestamp_intervals);
        println!("  mode (most common interval): {}s", most_common);
    } else {
        println!("  no intervals (only 1 snapshot per player or no snapshots)");
    }
    println!();
    println!("--- CMsgMatchPlayerPathsData ---");
    println!("  paths_player_count: {}", r.paths_player_count);
    println!("  interval_s:         {:?}", r.paths_interval_s);
    println!("  x_resolution:       {:?}", r.paths_x_resolution);
    println!("  y_resolution:       {:?}", r.paths_y_resolution);
    for ps in &r.path_summaries {
        println!(
            "  slot={} samples={} health={} combat_type={} move_type={} bbox=x[{:.1},{:.1}] y[{:.1},{:.1}]",
            ps.player_slot,
            ps.sample_count,
            ps.health_count,
            ps.combat_type_count,
            ps.move_type_count,
            ps.x_min,
            ps.x_max,
            ps.y_min,
            ps.y_max
        );
    }
    if let Some(cs) = &r.first_path_coord_sample {
        println!();
        println!("--- Coordinate decode validation (first path player slot={}) ---", cs.player_slot);
        println!("  x_min={} x_max={} x_resolution={}", cs.x_min, cs.x_max, cs.x_resolution);
        println!("  y_min={} y_max={} y_resolution={}", cs.y_min, cs.y_max, cs.y_resolution);
        println!("  raw: x_pos[0]={} y_pos[0]={}", cs.x_raw, cs.y_raw);
        println!("  decoded (hypothesis: x_world = x_min + (x_raw/x_res)*(x_max-x_min)):");
        println!("    x_world={:.2}  y_world={:.2}", cs.x_world, cs.y_world);
        println!("  (Deadlock world coords: roughly -7000..7000 on X, -7000..7000 on Y at match start)");
    }
    println!();
    println!("--- CMsgMatchPlayerDamageMatrix ---");
    println!("  damage_dealer_count: {}", r.damage_dealer_count);
    println!("  sample_time_s count: {}", r.damage_sample_times.len());
    if r.damage_sample_times.len() <= 10 {
        println!("  sample_time_s values: {:?}", r.damage_sample_times);
    } else {
        let first5 = &r.damage_sample_times[..5];
        let last5 = &r.damage_sample_times[r.damage_sample_times.len() - 5..];
        println!("  sample_time_s first5: {:?}", first5);
        println!("  sample_time_s last5:  {:?}", last5);
        // Compute intervals in sample_time_s
        let intervals: Vec<u32> = r.damage_sample_times
            .windows(2)
            .map(|w| w[1].saturating_sub(w[0]))
            .collect();
        let mut sorted_intervals = intervals.clone();
        sorted_intervals.sort();
        sorted_intervals.dedup();
        println!("  sample_time_s unique intervals: {:?}", sorted_intervals);
    }
    println!("  source_names count: {}", r.damage_source_names.len());
    if !r.damage_source_names.is_empty() {
        let sample_count = r.damage_source_names.len().min(10);
        println!("  source_names sample (first {}): {:?}", sample_count, &r.damage_source_names[..sample_count]);
    }
    println!();
    println!("--- MidBoss records (open question #5) ---");
    for (i, mb) in r.mid_boss_records.iter().enumerate() {
        println!(
            "  mid_boss[{}]: killed_by=team{:?} claimed_by=team{:?} at_s={:?} -- STEAL={}",
            i,
            mb.team_killed,
            mb.team_claimed,
            mb.destroyed_time_s,
            mb.is_steal
        );
    }
    if r.mid_boss_records.iter().any(|mb| mb.is_steal) {
        println!("  --> At least one steal observed: team_killed != team_claimed CONFIRMED");
    } else if !r.mid_boss_records.is_empty() {
        println!("  --> No steals in this replay (team_killed == team_claimed for all)");
    } else {
        println!("  --> No mid_boss kills in this replay");
    }
    println!();
}

fn mode(vals: &[u32]) -> u32 {
    let mut counts: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for &v in vals {
        *counts.entry(v).or_insert(0) += 1;
    }
    counts.into_iter().max_by_key(|&(_, c)| c).map(|(v, _)| v).unwrap_or(0)
}

async fn run_probe(path: &str) -> Result<Option<ReplayResult>> {
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();
    let file = File::open(path).with_context(|| format!("opening {}", path))?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)
        .with_context(|| format!("reading demo header for {}", path))?;
    let mut probe = Probe::new(name);
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut probe)
        .with_context(|| format!("creating parser for {}", path))?;
    parser
        .run_to_end()
        .await
        .with_context(|| format!("running parser for {}", path))?;
    Ok(probe.result)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "Usage: probe_post_match_details <replay1.dem> [replay2.dem ...]"
        );
        std::process::exit(1);
    }

    let mut results: Vec<ReplayResult> = Vec::new();
    for path in &args {
        println!("[probe] Parsing: {}", path);
        match run_probe(path).await {
            Ok(Some(r)) => results.push(r),
            Ok(None) => println!("[probe] WARNING: no PostMatchDetails found in {}", path),
            Err(e) => eprintln!("[probe] ERROR parsing {}: {:#}", path, e),
        }
    }

    println!();
    println!("========================================");
    println!("  PROBE REPORT: probe_post_match_details");
    println!("  Replays processed: {}", results.len());
    println!("========================================");
    println!();

    for r in &results {
        print_result(r);
        println!("----------------------------------------");
        println!();
    }

    // Cross-replay summary for open questions
    println!("========================================");
    println!("  CROSS-REPLAY SUMMARY");
    println!("========================================");
    println!();
    println!("Open question #1 -- assigned_lane encoding:");
    for r in &results {
        let lanes: Vec<Option<u32>> = r.player_summaries.iter().map(|p| p.assigned_lane).collect();
        let all_present = lanes.iter().all(|l| l.is_some());
        let values: Vec<u32> = lanes.into_iter().flatten().collect();
        let mut uniq = values.clone();
        uniq.sort();
        uniq.dedup();
        println!("  {}: all_present={} unique_values={:?}", r.replay_name, all_present, uniq);
    }
    println!(
        "  CMsgLaneColor: Invalid=0, Yellow=1, Green=3, Blue=4, Purple=6 (non-contiguous)"
    );
    println!("  If values are in [1,2,3,4] => sequential 1-indexed");
    println!("  If values are in [1,3,4,6] => CMsgLaneColor encoding");
    println!("  If values are in [0,1,2,3] => sequential 0-indexed");
    println!();
    println!("Open question #4 -- PlayerStats snapshot interval:");
    let mut all_intervals: Vec<u32> = Vec::new();
    for r in &results {
        all_intervals.extend_from_slice(&r.all_timestamp_intervals);
    }
    if !all_intervals.is_empty() {
        let mut sorted = all_intervals.clone();
        sorted.sort();
        sorted.dedup();
        println!("  All unique interval values across all replays: {:?}", sorted);
        println!("  Mode: {}s", mode(&all_intervals));
    }
    println!();
    println!("Open question #5 -- mid_boss steal mechanic:");
    let any_steal = results
        .iter()
        .any(|r| r.mid_boss_records.iter().any(|mb| mb.is_steal));
    let total_kills: usize = results.iter().map(|r| r.mid_boss_records.len()).sum();
    println!("  Total mid_boss kills across all replays: {}", total_kills);
    println!("  Any steals observed (team_killed != team_claimed): {}", any_steal);
    println!();
    println!("Open question #3 -- coordinate decode formula:");
    println!("  Hypothesis: x_world = x_min + (x_raw / x_resolution) * (x_max - x_min)");
    for r in &results {
        if let Some(cs) = &r.first_path_coord_sample {
            println!(
                "  {} slot={}: x_world={:.2} y_world={:.2}",
                r.replay_name, cs.player_slot, cs.x_world, cs.y_world
            );
        }
    }
    println!("  (Expected: near spawn/base at match second 0 -- approx x in [-7000,7000], y in [-7000,7000])");
    println!();

    Ok(())
}
