//! Diagnostic binary: probe CCitadelUserMsg_MidBossSpawned (349), CCitadelUserMsg_BossKilled (347),
//! CCitadelUserMsg_RejuvStatus (350), CCitadelUserMsg_BossDamaged (348), and
//! CCitadelUserMsgHudGameAnnouncement (363).
//!
//! Runs a full replay parse and logs every field for each message event, plus ctx.tick().
//! Summary output at the end covers:
//! - MidBossSpawned event count
//! - BossKilled event count with entity_killed_class distribution
//! - Spawn-to-kill tick deltas and seconds (for respawn timer validation, Q3)
//! - RejuvStatus event_type distribution per kill (Q6)
//! - entity_killed_class values observed on BossKilled (Q5)
//! - HudGameAnnouncement events (candidate for 50% roar, Q4)
//! - MidBossSpawned fire count vs expected (Q7)
//!
//! Usage (from worktree root):
//!   docker compose --project-directory ../dashjump-gg-midboss exec dashjump-parser \
//!     cargo run --bin probe_midboss_runtime -- /parser/src/replays/<file>.dem

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity, fkey_from_path};
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::deadlock::{
    CitadelUserMessageIds,
    CCitadelUserMsgBossKilled,
    CCitadelUserMsgMidBossSpawned,
    CCitadelUserMsgRejuvStatus,
    CCitadelUserMsgBossDamaged,
    CCitadelUserMsgHudGameAnnouncement,
};
use prost::Message;

const DEADLOCK_GAMERULES_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");
const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);

const CNPC_MIDBOSS_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_MidBoss");
const HEALTH_KEY: u64 = fkey_from_path(&["m_iHealth"]);
const MAX_HEALTH_KEY: u64 = fkey_from_path(&["m_iMaxHealth"]);

/// Thin error wrapper matching the Visitor::Error bound used in replay_parser.rs.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

impl From<prost::DecodeError> for ProbeError {
    fn from(e: prost::DecodeError) -> Self {
        ProbeError(anyhow::Error::from(e))
    }
}

/// One recorded CNPC_MidBoss health observation (logged when m_iMaxHealth changes).
#[derive(Debug, Clone)]
struct HealthObservation {
    tick: i32,
    match_time_s: f32,
    health: i32,
    max_health: i32,
}

/// One recorded spawn event: tick at which MidBossSpawned fired.
#[derive(Debug, Clone)]
struct SpawnRecord {
    tick: i32,
    match_time_s: f32,
}

/// One recorded kill event from BossKilled.
#[derive(Debug, Clone)]
struct KillRecord {
    tick: i32,
    entity_killed_class: i32,
    objective_team: i32,
    objective_mask_change: i32,
    entity_killed: u32,
    entity_killer: u32,
    gametime: f32,
    bosses_remaining: i32,
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
}

/// One recorded RejuvStatus event.
#[derive(Debug, Clone)]
struct RejuvRecord {
    tick: i32,
    killing_team: i32,
    player_pawn: u32,
    user_team: i32,
    event_type: i32,
}

struct ProbeVisitor {
    match_start_time_s: Option<f32>,
    tick_interval: f32,

    spawns: Vec<SpawnRecord>,
    kills: Vec<KillRecord>,
    rejuvs: Vec<RejuvRecord>,

    total_boss_damaged: u64,
    total_hud_announcements: u64,

    health_observations: Vec<HealthObservation>,
    last_max_health: Option<i32>,
}

impl Default for ProbeVisitor {
    fn default() -> Self {
        Self {
            match_start_time_s: None,
            tick_interval: 1.0 / 60.0, // Deadlock is 60 tps; confirmed at runtime via ctx
            spawns: Vec::new(),
            kills: Vec::new(),
            rejuvs: Vec::new(),
            total_boss_damaged: 0,
            total_hud_announcements: 0,
            health_observations: Vec::new(),
            last_max_health: None,
        }
    }
}

impl ProbeVisitor {
    fn match_time_s(&self, tick: i32) -> f32 {
        let raw = tick as f32 * self.tick_interval;
        raw - self.match_start_time_s.unwrap_or(0.0)
    }

    fn handle_mid_boss_spawned(&mut self, ctx: &Context, _msg: CCitadelUserMsgMidBossSpawned) {
        let tick = ctx.tick();
        let match_time_s = self.match_time_s(tick);
        println!("[tick={tick}] MidBossSpawned  match_time_s={match_time_s:.3}s");
        self.spawns.push(SpawnRecord { tick, match_time_s });
    }

    fn handle_boss_killed(&mut self, ctx: &Context, msg: CCitadelUserMsgBossKilled) {
        let tick = ctx.tick();
        let entity_killed_class = msg.entity_killed_class.unwrap_or(-1);
        let objective_team = msg.objective_team.unwrap_or(-1);
        let objective_mask_change = msg.objective_mask_change.unwrap_or(-1);
        let entity_killed = msg.entity_killed.unwrap_or(0xFFFFFF);
        let entity_killer = msg.entity_killer.unwrap_or(0xFFFFFF);
        let gametime = msg.gametime.unwrap_or(-1.0);
        let bosses_remaining = msg.bosses_remaining.unwrap_or(-1);
        let (pos_x, pos_y, pos_z) = msg.entity_position
            .map(|p| (p.x.unwrap_or(0.0), p.y.unwrap_or(0.0), p.z.unwrap_or(0.0)))
            .unwrap_or((0.0, 0.0, 0.0));

        println!(
            "[tick={tick}] BossKilled  class={entity_killed_class} team={objective_team} \
             mask_change={objective_mask_change} entity_killed={entity_killed:#010x} \
             entity_killer={entity_killer:#010x} gametime={gametime:.3} \
             bosses_remaining={bosses_remaining} pos=({pos_x:.1},{pos_y:.1},{pos_z:.1})"
        );

        self.kills.push(KillRecord {
            tick,
            entity_killed_class,
            objective_team,
            objective_mask_change,
            entity_killed,
            entity_killer,
            gametime,
            bosses_remaining,
            pos_x,
            pos_y,
            pos_z,
        });
    }

    fn handle_rejuv_status(&mut self, ctx: &Context, msg: CCitadelUserMsgRejuvStatus) {
        let tick = ctx.tick();
        let killing_team = msg.killing_team.unwrap_or(-1);
        let player_pawn = msg.player_pawn.unwrap_or(0xFFFFFF);
        let user_team = msg.user_team.unwrap_or(-1);
        let event_type = msg.event_type.unwrap_or(-1);

        println!(
            "[tick={tick}] RejuvStatus  killing_team={killing_team} user_team={user_team} \
             player_pawn={player_pawn:#010x} event_type={event_type}"
        );

        self.rejuvs.push(RejuvRecord {
            tick,
            killing_team,
            player_pawn,
            user_team,
            event_type,
        });
    }

    fn handle_boss_damaged(&mut self, _ctx: &Context, _msg: CCitadelUserMsgBossDamaged) {
        self.total_boss_damaged += 1;
    }

    fn handle_hud_announcement(&mut self, ctx: &Context, msg: CCitadelUserMsgHudGameAnnouncement) {
        let tick = ctx.tick();
        let title = msg.title_locstring.as_deref().unwrap_or("");
        let desc = msg.description_locstring.as_deref().unwrap_or("");
        let classnames = msg.classname.join(", ");
        println!(
            "[tick={tick}] HudGameAnnouncement  title=\"{title}\" desc=\"{desc}\" \
             classnames=[{classnames}]"
        );
        self.total_hud_announcements += 1;
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
        if entity.serializer().serializer_name.hash == DEADLOCK_GAMERULES_ENTITY {
            if let Ok(t) = entity.try_get_value::<f32>(&GAME_START_KEY) {
                if t > 0.001 && self.match_start_time_s.is_none() {
                    self.match_start_time_s = Some(t);
                    self.tick_interval = ctx.tick_interval();
                    println!(
                        "[setup] match_start_time_s={t:.4}s  tick_interval={:.6}  tps={:.1}",
                        ctx.tick_interval(),
                        1.0 / ctx.tick_interval()
                    );
                }
            }
        }

        if entity.serializer().serializer_name.hash == CNPC_MIDBOSS_ENTITY {
            match delta_header {
                DeltaHeader::CREATE => {
                    println!("[tick={}] CNPC_MidBoss CREATE", ctx.tick());
                }
                DeltaHeader::DELETE => {
                    println!("[tick={}] CNPC_MidBoss DELETE", ctx.tick());
                    self.last_max_health = None;
                }
                _ => {}
            }

            let health = entity.try_get_value::<i32>(&HEALTH_KEY).unwrap_or(-1);
            let max_health = entity.try_get_value::<i32>(&MAX_HEALTH_KEY).unwrap_or(-1);
            if max_health > 0 && Some(max_health) != self.last_max_health {
                let match_time_s = self.match_time_s(ctx.tick());
                println!(
                    "[tick={}] CNPC_MidBoss m_iHealth={} m_iMaxHealth={} match_time_s={:.3}s",
                    ctx.tick(), health, max_health, match_time_s
                );
                self.health_observations.push(HealthObservation {
                    tick: ctx.tick(),
                    match_time_s,
                    health,
                    max_health,
                });
                self.last_max_health = Some(max_health);
            }
        }

        Ok(())
    }

    async fn on_packet(
        &mut self,
        ctx: &Context,
        packet_type: u32,
        data: &[u8],
    ) -> Result<(), ProbeError> {
        if packet_type == CitadelUserMessageIds::KEUserMsgMidBossSpawned as u32 {
            let msg = CCitadelUserMsgMidBossSpawned::decode(data)?;
            self.handle_mid_boss_spawned(ctx, msg);
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgBossKilled as u32 {
            let msg = CCitadelUserMsgBossKilled::decode(data)?;
            self.handle_boss_killed(ctx, msg);
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgRejuvStatus as u32 {
            let msg = CCitadelUserMsgRejuvStatus::decode(data)?;
            self.handle_rejuv_status(ctx, msg);
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgBossDamaged as u32 {
            let msg = CCitadelUserMsgBossDamaged::decode(data)?;
            self.handle_boss_damaged(ctx, msg);
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgHudGameAnnouncement as u32 {
            let msg = CCitadelUserMsgHudGameAnnouncement::decode(data)?;
            self.handle_hud_announcement(ctx, msg);
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, _ctx: &Context) -> Result<(), ProbeError> { Ok(()) }

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
    let path = env::args().nth(1).expect("Usage: probe_midboss_runtime <replay.dem>");

    eprintln!("Probing: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = ProbeVisitor::default();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    let tick_interval = visitor.tick_interval;

    // ========== SUMMARY ==========
    println!("\n========== MID-BOSS RUNTIME PROBE SUMMARY ==========");
    println!("MidBossSpawned events:     {}", visitor.spawns.len());
    println!("BossKilled events (total): {}", visitor.kills.len());
    println!("RejuvStatus events:        {}", visitor.rejuvs.len());
    println!("BossDamaged events:        {}", visitor.total_boss_damaged);
    println!("HudGameAnnouncement:       {}", visitor.total_hud_announcements);
    println!("tick_interval confirmed:   {tick_interval:.6}  (tps={:.1})", 1.0 / tick_interval);

    // -- entity_killed_class distribution (Q5)
    println!("\n-- BossKilled entity_killed_class distribution (Q5) --");
    let mut class_tally: HashMap<i32, u32> = HashMap::new();
    for kill in &visitor.kills {
        *class_tally.entry(kill.entity_killed_class).or_default() += 1;
    }
    let mut class_list: Vec<(i32, u32)> = class_tally.into_iter().collect();
    class_list.sort_by_key(|(c, _)| *c);
    for (class, count) in &class_list {
        println!("  entity_killed_class={class:>5} : {count} kills");
    }

    // -- BossKilled event details
    println!("\n-- BossKilled event details --");
    for (i, kill) in visitor.kills.iter().enumerate() {
        println!(
            "  kill[{i}] tick={} class={} team={} gametime={:.3}s bosses_remaining={} \
             pos=({:.1},{:.1},{:.1})",
            kill.tick, kill.entity_killed_class, kill.objective_team,
            kill.gametime, kill.bosses_remaining,
            kill.pos_x, kill.pos_y, kill.pos_z
        );
    }

    // -- MidBossSpawned event details (Q7)
    println!("\n-- MidBossSpawned event details (Q7) --");
    for (i, spawn) in visitor.spawns.iter().enumerate() {
        println!("  spawn[{i}] tick={} match_time_s={:.3}s", spawn.tick, spawn.match_time_s);
    }

    // -- Spawn-to-kill deltas (Q3: respawn timer validation)
    // Pair each kill with the next spawn after it to compute respawn delay.
    println!("\n-- Spawn-to-kill deltas (Q3: respawn timer validation) --");
    if visitor.spawns.is_empty() {
        println!("  No spawn events recorded.");
    } else {
        let spawn_ticks: Vec<i32> = {
            let mut v: Vec<i32> = visitor.spawns.iter().map(|s| s.tick).collect();
            v.sort();
            v
        };

        // First spawn -> first kill lifetime
        if let Some(first_kill) = visitor.kills.first() {
            let first_spawn_tick = spawn_ticks[0];
            if first_kill.tick > first_spawn_tick {
                let delta_ticks = first_kill.tick - first_spawn_tick;
                let delta_s = delta_ticks as f32 * tick_interval;
                let delta_min = delta_s / 60.0;
                println!(
                    "  spawn[0] tick={} -> kill[0] tick={}  lifetime={delta_ticks} ticks \
                     = {delta_s:.1}s = {delta_min:.2} min",
                    first_spawn_tick, first_kill.tick
                );
            }
        }

        // Kill -> next spawn (= respawn delay)
        for (ki, kill) in visitor.kills.iter().enumerate() {
            if let Some(&next_spawn) = spawn_ticks.iter().find(|&&t| t > kill.tick) {
                let delta_ticks = next_spawn - kill.tick;
                let delta_s = delta_ticks as f32 * tick_interval;
                let delta_min = delta_s / 60.0;
                println!(
                    "  kill[{ki}] tick={} -> next_spawn tick={next_spawn}  \
                     respawn_delay={delta_ticks} ticks = {delta_s:.1}s = {delta_min:.2} min  \
                     (class={})",
                    kill.tick, kill.entity_killed_class
                );
            } else {
                println!(
                    "  kill[{ki}] tick={} -> no next spawn after this kill (class={})",
                    kill.tick, kill.entity_killed_class
                );
            }
        }
    }

    // -- RejuvStatus event_type distribution (Q6)
    println!("\n-- RejuvStatus event_type distribution (Q6) --");
    if visitor.rejuvs.is_empty() {
        println!("  No RejuvStatus events recorded.");
    } else {
        let mut type_tally: HashMap<i32, u32> = HashMap::new();
        for rejuv in &visitor.rejuvs {
            *type_tally.entry(rejuv.event_type).or_default() += 1;
        }
        let mut type_list: Vec<(i32, u32)> = type_tally.into_iter().collect();
        type_list.sort_by_key(|(t, _)| *t);
        for (event_type, count) in &type_list {
            println!("  event_type={event_type:>3} : {count} events");
        }
    }

    // -- RejuvStatus events grouped near each kill (by tick proximity, window = 600 ticks = ~10s)
    println!("\n-- RejuvStatus events near each kill (within 600 ticks) --");
    for (ki, kill) in visitor.kills.iter().enumerate() {
        let near: Vec<&RejuvRecord> = visitor.rejuvs.iter()
            .filter(|r| r.tick >= kill.tick && r.tick <= kill.tick + 600)
            .collect();
        if near.is_empty() {
            println!("  kill[{ki}] (class={}) tick={} -> 0 nearby RejuvStatus events",
                kill.entity_killed_class, kill.tick);
        } else {
            println!("  kill[{ki}] (class={}) tick={} -> {} nearby RejuvStatus events:",
                kill.entity_killed_class, kill.tick, near.len());
            for r in near {
                println!("    tick={} killing_team={} user_team={} event_type={} pawn={:#010x}",
                    r.tick, r.killing_team, r.user_team, r.event_type, r.player_pawn);
            }
        }
    }

    // -- Q4: 50% roar detection summary
    println!("\n-- Q4: 50% health roar event --");
    println!("  CCitadelUserMsgBossDamaged (ID 348) total events: {}", visitor.total_boss_damaged);
    println!("  This message fires on boss damage but has no health field -- cannot detect 50% directly.");
    println!("  CCitadelUserMsgHudGameAnnouncement (ID 363) total: {}  (see log above for content)", visitor.total_hud_announcements);

    // -- Q7: MidBossSpawned reliability
    println!("\n-- Q7: MidBossSpawned reliability --");
    let kill_count = visitor.kills.len();
    let spawn_count = visitor.spawns.len();
    println!("  Spawn events: {spawn_count}  Kill events: {kill_count}");
    println!("  Expected spawn count = kill_count + 1 (if boss alive at match end) or kill_count (if not)");
    if spawn_count == kill_count + 1 {
        println!("  -> Pattern: boss alive at match end (spawned once more than killed) -- consistent with single-fire");
    } else if spawn_count == kill_count {
        println!("  -> Pattern: boss killed exactly as many times as it spawned -- consistent with single-fire");
    } else {
        println!("  -> UNEXPECTED: spawn/kill count mismatch -- investigate for double-fire or silent spawn");
    }

    // -- Q1: CNPC_MidBoss m_iMaxHealth timeline (scaling formula validation)
    println!("\n-- CNPC_MidBoss m_iMaxHealth timeline (Q1: scaling formula) --");
    if visitor.health_observations.is_empty() {
        println!("  No CNPC_MidBoss health observations recorded (entity may not use m_iMaxHealth).");
    } else {
        for obs in &visitor.health_observations {
            println!(
                "  tick={} match_time_s={:.3}s health={} max_health={}",
                obs.tick, obs.match_time_s, obs.health, obs.max_health
            );
        }
    }

    // -- Q1 analysis: initial max_health per spawn cycle
    println!("\n-- Q1: initial max_health at each MidBossSpawned tick --");
    let spawn_ticks_sorted: Vec<i32> = {
        let mut v: Vec<i32> = visitor.spawns.iter().map(|s| s.tick).collect();
        v.sort();
        v
    };
    for (si, &spawn_tick) in spawn_ticks_sorted.iter().enumerate() {
        // Find the first health observation at or after this spawn tick
        // (but before the next spawn tick, if any)
        let next_spawn = spawn_ticks_sorted.get(si + 1).copied().unwrap_or(i32::MAX);
        if let Some(first_obs) = visitor.health_observations.iter()
            .find(|o| o.tick >= spawn_tick && o.tick < next_spawn)
        {
            let minutes = first_obs.match_time_s / 60.0;
            let wiki_formula = 13000.0 + 195.0 * minutes;
            println!(
                "  spawn[{si}] tick={spawn_tick}: first obs tick={} match_time_s={:.3}s \
                 max_health={} (wiki formula@{:.2}min = {:.0}, delta = {:.0})",
                first_obs.tick, first_obs.match_time_s, first_obs.max_health,
                minutes, wiki_formula,
                first_obs.max_health as f32 - wiki_formula
            );
        } else {
            println!("  spawn[{si}] tick={spawn_tick}: no health observation found in window");
        }
    }

    // -- Q1 analysis: slope between consecutive max_health observations
    println!("\n-- Q1: max_health slope between consecutive observations --");
    if visitor.health_observations.len() >= 2 {
        for i in 1..visitor.health_observations.len() {
            let prev = &visitor.health_observations[i - 1];
            let curr = &visitor.health_observations[i];
            let dt_s = curr.match_time_s - prev.match_time_s;
            let dt_min = dt_s / 60.0;
            let d_max = curr.max_health - prev.max_health;
            let slope_per_min = if dt_min > 0.0 { d_max as f32 / dt_min } else { 0.0 };
            println!(
                "  obs[{}->{}] dt={:.3}s ({:.3}min) d_max_health={} slope={:.1}/min",
                i - 1, i, dt_s, dt_min, d_max, slope_per_min
            );
        }
    } else {
        println!("  Not enough observations to compute slope.");
    }

    Ok(())
}
