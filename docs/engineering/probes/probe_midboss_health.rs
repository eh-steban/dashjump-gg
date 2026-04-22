//! Diagnostic binary: sample CNPC_MidBoss m_iHealth / m_iMaxHealth on every entity
//! UPDATE (gated to `tick % 60 == 0` -> 1s granularity floor) plus CREATE/DELETE,
//! and correlate with CCitadelUserMsg_MidBossSpawned (349) / CCitadelUserMsg_BossKilled
//! (347) / CCitadelUserMsg_BossDamaged (348).
//!
//! Goal: answer three questions for the mid-boss health-bug fix plan.
//!   Q1: Is `m_iMaxHealth` dynamic within one CREATE lifetime (scales over time
//!       from spawn to death), or is it a static value set at CREATE?
//!   Q2: Does `m_iHealth` regen show up as UPDATE deltas between damage events?
//!       Rate should approach 15 HP/s per the wiki.
//!   Q3: Which scaling formula fits the observed max_health at each spawn cycle:
//!         A) 13000 + 195 * match_time_minutes              (match-time formula, spike)
//!         B) 13000 + 195 * (match_time - spawn_time)/60    (since-spawn formula, user)
//!       Note: wiki implies B but initial cycle happens immediately at spawn, so
//!       both formulas collapse at cycle 1 if spawn_time == match_time at first
//!       observation. They diverge on cycles 2/3 (respawn after kill).
//!
//! Usage (from worktree root):
//!   docker compose \
//!     --file /home/lifted/Code/dashjump/dashjump-gg/docker-compose.yaml \
//!     --file /home/lifted/Code/dashjump/dashjump-gg-midboss/docker-compose.override.yaml \
//!     --project-directory /home/lifted/Code/dashjump/dashjump-gg-midboss \
//!     exec -T dashjump-parser \
//!     cargo run --bin probe_midboss_health -- /parser/src/replays/<file>.dem

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
    CCitadelUserMsgBossDamaged,
};
use prost::Message;

const DEADLOCK_GAMERULES_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");
const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);

const CNPC_MIDBOSS_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_MidBoss");
const HEALTH_KEY: u64 = fkey_from_path(&["m_iHealth"]);
const MAX_HEALTH_KEY: u64 = fkey_from_path(&["m_iMaxHealth"]);

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ProbeError(#[from] anyhow::Error);

impl From<prost::DecodeError> for ProbeError {
    fn from(e: prost::DecodeError) -> Self {
        ProbeError(anyhow::Error::from(e))
    }
}

/// One per-second sample of mid-boss health during a single entity lifetime.
#[derive(Debug, Clone)]
struct HealthSample {
    cycle: u32,               // 1-indexed lifetime (CREATE instance)
    tick: i32,
    match_time_s: f32,
    health: i32,
    max_health: i32,
    is_create: bool,
    is_delete: bool,
}

/// Recorded MidBossSpawned message (349).
#[derive(Debug, Clone)]
struct SpawnMsg {
    tick: i32,
    match_time_s: f32,
}

/// Recorded BossKilled message (347), filtered to class==17 (mid-boss) later.
#[derive(Debug, Clone)]
struct KillMsg {
    tick: i32,
    match_time_s: f32,
    entity_killed_class: i32,
}

/// Recorded BossDamaged message (348) count per second (for regen-between-damage analysis).
#[derive(Debug, Clone)]
struct DamageMsg {
    tick: i32,
    match_time_s: f32,
}

struct ProbeVisitor {
    match_start_time_s: Option<f32>,
    tick_interval: f32,

    spawns: Vec<SpawnMsg>,
    kills: Vec<KillMsg>,
    damage: Vec<DamageMsg>,

    samples: Vec<HealthSample>,

    /// Incremented on each CNPC_MidBoss CREATE; sample.cycle field.
    current_cycle: u32,
    /// True while a CNPC_MidBoss instance is alive (between CREATE and DELETE).
    in_lifetime: bool,
}

impl Default for ProbeVisitor {
    fn default() -> Self {
        Self {
            match_start_time_s: None,
            tick_interval: 1.0 / 60.0, // Deadlock default; overridden at runtime via ctx.
            spawns: Vec::new(),
            kills: Vec::new(),
            damage: Vec::new(),
            samples: Vec::new(),
            current_cycle: 0,
            in_lifetime: false,
        }
    }
}

impl ProbeVisitor {
    fn match_time_s(&self, tick: i32) -> f32 {
        let raw = tick as f32 * self.tick_interval;
        raw - self.match_start_time_s.unwrap_or(0.0)
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
            let tick = ctx.tick();
            let match_time_s = self.match_time_s(tick);
            let health = entity.try_get_value::<i32>(&HEALTH_KEY).unwrap_or(-1);
            let max_health = entity.try_get_value::<i32>(&MAX_HEALTH_KEY).unwrap_or(-1);

            match delta_header {
                DeltaHeader::CREATE => {
                    self.current_cycle += 1;
                    self.in_lifetime = true;
                    println!(
                        "[tick={tick}] CNPC_MidBoss CREATE cycle={} health={} max_health={} match_time_s={:.3}",
                        self.current_cycle, health, max_health, match_time_s
                    );
                    self.samples.push(HealthSample {
                        cycle: self.current_cycle,
                        tick,
                        match_time_s,
                        health,
                        max_health,
                        is_create: true,
                        is_delete: false,
                    });
                }
                DeltaHeader::DELETE => {
                    println!(
                        "[tick={tick}] CNPC_MidBoss DELETE cycle={} health={} max_health={} match_time_s={:.3}",
                        self.current_cycle, health, max_health, match_time_s
                    );
                    self.samples.push(HealthSample {
                        cycle: self.current_cycle,
                        tick,
                        match_time_s,
                        health,
                        max_health,
                        is_create: false,
                        is_delete: true,
                    });
                    self.in_lifetime = false;
                }
                _ => {
                    // UPDATE: sample at 1-second cadence while alive.
                    if self.in_lifetime && tick % 60 == 0 && max_health > 0 {
                        self.samples.push(HealthSample {
                            cycle: self.current_cycle,
                            tick,
                            match_time_s,
                            health,
                            max_health,
                            is_create: false,
                            is_delete: false,
                        });
                    }
                }
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
            let _msg = CCitadelUserMsgMidBossSpawned::decode(data)?;
            let tick = ctx.tick();
            let match_time_s = self.match_time_s(tick);
            println!("[tick={tick}] MidBossSpawned  match_time_s={match_time_s:.3}s");
            self.spawns.push(SpawnMsg { tick, match_time_s });
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgBossKilled as u32 {
            let msg = CCitadelUserMsgBossKilled::decode(data)?;
            let tick = ctx.tick();
            let match_time_s = self.match_time_s(tick);
            let cls = msg.entity_killed_class.unwrap_or(-1);
            // Class 17 is mid-boss per prior probe runs; log all classes but tag later.
            println!(
                "[tick={tick}] BossKilled  class={cls}  match_time_s={match_time_s:.3}"
            );
            self.kills.push(KillMsg { tick, match_time_s, entity_killed_class: cls });
        }
        if packet_type == CitadelUserMessageIds::KEUserMsgBossDamaged as u32 {
            let _msg = CCitadelUserMsgBossDamaged::decode(data)?;
            let tick = ctx.tick();
            let match_time_s = self.match_time_s(tick);
            self.damage.push(DamageMsg { tick, match_time_s });
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
    let path = env::args().nth(1).expect("Usage: probe_midboss_health <replay.dem>");

    eprintln!("Probing: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;

    let mut visitor = ProbeVisitor::default();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    println!("\n========== MID-BOSS HEALTH PROBE SUMMARY ==========");
    println!("MidBossSpawned msgs:  {}", visitor.spawns.len());
    println!("BossKilled msgs:      {}", visitor.kills.len());
    println!("BossDamaged msgs:     {}", visitor.damage.len());
    println!("Health samples:       {}  (CREATE + DELETE + UPDATE@1s)", visitor.samples.len());
    println!("tick_interval:        {:.6}  (tps={:.1})", visitor.tick_interval, 1.0 / visitor.tick_interval);

    // ---- Full per-second dump ----------------------------------------------
    println!("\n========== FULL SAMPLE DUMP (one row per CREATE/DELETE + 1s tick) ==========");
    println!("  cycle  kind         tick  match_time_s   mm:ss    health  max_health   pct");
    for s in &visitor.samples {
        let kind = if s.is_create { "CREATE " }
                   else if s.is_delete { "DELETE " }
                   else { "UPDATE " };
        let mm = (s.match_time_s / 60.0).floor() as i32;
        let ss = s.match_time_s - (mm as f32 * 60.0);
        let pct = if s.max_health > 0 {
            100.0 * (s.health as f32) / (s.max_health as f32)
        } else { 0.0 };
        println!(
            "  {:>5}  {kind}  {:>6}  {:>11.3}   {:>02}:{:05.2}  {:>6}  {:>10}   {:>5.1}%",
            s.cycle, s.tick, s.match_time_s, mm, ss, s.health, s.max_health, pct
        );
    }

    // ---- Per-cycle breakdown ------------------------------------------------
    let cycles: Vec<u32> = {
        let mut cs: Vec<u32> = visitor.samples.iter().map(|s| s.cycle).collect();
        cs.sort();
        cs.dedup();
        cs
    };

    for c in &cycles {
        let cycle_samples: Vec<&HealthSample> =
            visitor.samples.iter().filter(|s| s.cycle == *c).collect();
        if cycle_samples.is_empty() { continue; }

        let create = cycle_samples.iter().find(|s| s.is_create);
        let delete = cycle_samples.iter().find(|s| s.is_delete);
        let updates: Vec<&&HealthSample> = cycle_samples.iter()
            .filter(|s| !s.is_create && !s.is_delete)
            .collect();

        println!("\n---- Cycle {c} ----");
        if let Some(c0) = create {
            println!(
                "  CREATE   tick={} match_time_s={:.3}  health={}  max_health={}",
                c0.tick, c0.match_time_s, c0.health, c0.max_health
            );
        }
        if let Some(d0) = delete {
            println!(
                "  DELETE   tick={} match_time_s={:.3}  health={}  max_health={}",
                d0.tick, d0.match_time_s, d0.health, d0.max_health
            );
        }

        // Q1: does max_health change during this cycle's lifetime?
        let mut max_h_vals: Vec<i32> = cycle_samples.iter()
            .filter(|s| s.max_health > 0)
            .map(|s| s.max_health)
            .collect();
        max_h_vals.sort();
        max_h_vals.dedup();
        println!("  distinct max_health values in cycle {c}: {max_h_vals:?}");

        // If max_health changed, show when.
        if max_h_vals.len() > 1 {
            println!("  >>> Q1: max_health DOES change during lifetime (dynamic scaling)");
            let mut prev: Option<i32> = None;
            for s in &cycle_samples {
                if s.max_health > 0 && Some(s.max_health) != prev {
                    println!(
                        "    change @ tick={} match_time_s={:.3}  max_health={}  (delta from prev: {})",
                        s.tick, s.match_time_s, s.max_health,
                        s.max_health - prev.unwrap_or(s.max_health)
                    );
                    prev = Some(s.max_health);
                }
            }
        } else {
            println!("  >>> Q1: max_health is STATIC for this cycle (= {:?})", max_h_vals.first());
        }

        // Q2: regen rate between UPDATE samples -- only interesting when health below max.
        println!("  -- regen analysis (health deltas between consecutive samples) --");
        let mut prev: Option<&&HealthSample> = None;
        let mut regen_segments: Vec<(f32, f32, i32)> = Vec::new(); // (dt_s, rate_hp_per_s, delta_h)
        for s in &updates {
            if let Some(p) = prev {
                let dt_s = s.match_time_s - p.match_time_s;
                let dh = s.health - p.health;
                if dt_s > 0.0 && dh > 0 && s.health < s.max_health {
                    let rate = dh as f32 / dt_s;
                    regen_segments.push((dt_s, rate, dh));
                }
            }
            prev = Some(s);
        }
        if regen_segments.is_empty() {
            println!("    no positive health deltas observed (boss either full-hp or monotonically damaged)");
        } else {
            println!("    observed {} positive-delta segments", regen_segments.len());
            let mut rates: Vec<f32> = regen_segments.iter().map(|(_, r, _)| *r).collect();
            rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = rates[rates.len()/2];
            let max_rate = rates.last().copied().unwrap_or(0.0);
            let min_rate = rates.first().copied().unwrap_or(0.0);
            println!("    rate hp/s: min={min_rate:.2}  median={median:.2}  max={max_rate:.2}");
            // Show first few segments for inspection.
            for (i, (dt, r, dh)) in regen_segments.iter().take(10).enumerate() {
                println!("      seg[{i}] dt={dt:.3}s  dh={dh}  rate={r:.2} hp/s");
            }
        }
    }

    // ---- Q3: scaling formula fit at each spawn message --------------------
    println!("\n========== Q3: scaling formula fit per MidBossSpawned ==========");
    println!("Formulas:");
    println!("  A (match-time)   : expected = 13000 + 195 * match_time_min");
    println!("  B (since-spawn)  : expected = 13000 + 195 * (match_time - spawn_time)/60");
    println!("                      -> at the spawn tick itself, expected = 13000");

    for (i, spawn) in visitor.spawns.iter().enumerate() {
        // Find the first CREATE sample at or after this spawn tick with the same cycle.
        let first_sample = visitor.samples.iter()
            .find(|s| s.is_create && s.tick >= spawn.tick);
        if let Some(fs) = first_sample {
            let match_time_min = fs.match_time_s / 60.0;
            let formula_a = 13000.0 + 195.0 * match_time_min;
            let since_spawn_min = (fs.match_time_s - spawn.match_time_s) / 60.0;
            let formula_b = 13000.0 + 195.0 * since_spawn_min.max(0.0);
            println!(
                "  spawn[{i}] match_time_s={:.3}  -> CREATE match_time_s={:.3}  max_health={}",
                spawn.match_time_s, fs.match_time_s, fs.max_health
            );
            println!(
                "            formula A = {:.0} (delta {:+.0})    formula B = {:.0} (delta {:+.0})",
                formula_a, fs.max_health as f32 - formula_a,
                formula_b, fs.max_health as f32 - formula_b
            );
        } else {
            println!("  spawn[{i}] match_time_s={:.3}  -> no CREATE sample found", spawn.match_time_s);
        }
    }

    // ---- Q3 supplemental: how max_health evolves WITHIN a cycle (if at all) ----
    println!("\n========== Q3 supplemental: max_health vs match_time within each cycle ==========");
    for c in &cycles {
        let cycle_samples: Vec<&HealthSample> = visitor.samples.iter()
            .filter(|s| s.cycle == *c && s.max_health > 0)
            .collect();
        if cycle_samples.len() < 2 { continue; }
        let first = cycle_samples.first().unwrap();
        let last = cycle_samples.last().unwrap();
        let dt_min = (last.match_time_s - first.match_time_s) / 60.0;
        let d_max = last.max_health - first.max_health;
        let slope = if dt_min > 0.0 { d_max as f32 / dt_min } else { 0.0 };
        println!(
            "  cycle {c}: first sample @ match_time_s={:.3} max_h={}  last @ match_time_s={:.3} max_h={}  slope={:.2}/min",
            first.match_time_s, first.max_health, last.match_time_s, last.max_health, slope
        );
    }

    Ok(())
}
