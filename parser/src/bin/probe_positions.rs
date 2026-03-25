/// Compare position field paths with and without m_skeletonInstance.m_vecOrigin.
///
/// Usage: cargo run --bin probe_positions -- <path-to.dem>
///
/// Prints one row per second for the first 10 match seconds showing:
///   - old path (CBodyComponent.m_skeletonInstance.m_vecOrigin.m_cellX) -- always None
///   - new path (CBodyComponent.m_cellX) -- correct values
///   - computed world coordinates from new path
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity, deadlock_coord_from_cell, fkey_from_path};
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};

const CCITADELPLAYERPAWN: u64 = fxhash::hash_bytes(b"CCitadelPlayerPawn");
const GAMERULES: u64 = fxhash::hash_bytes(b"CCitadelGameRulesProxy");

// Old path (from haste-inspector display -- wrong for get_value lookups)
const OLD_CX: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellX"]);
const OLD_CY: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellY"]);
const OLD_VX: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecX"]);
const OLD_VY: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecY"]);

// New path (send_node="CBodyComponent", var_name="m_cellX" -- matches stored key)
const NEW_CX: u64 = fkey_from_path(&["CBodyComponent", "m_cellX"]);
const NEW_CY: u64 = fkey_from_path(&["CBodyComponent", "m_cellY"]);
const NEW_VX: u64 = fkey_from_path(&["CBodyComponent", "m_vecX"]);
const NEW_VY: u64 = fkey_from_path(&["CBodyComponent", "m_vecY"]);

const GAME_START_KEY: u64 = fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ParseError(#[from] anyhow::Error);

struct ProbeVisitor {
    game_start_time: Option<f32>,
    tracked_entity_index: Option<i32>,
    last_printed_second: i32,
    printed: u32,
}

impl Visitor for &mut ProbeVisitor {
    type Error = ParseError;

    async fn on_packet(&mut self, _ctx: &Context, _t: u32, _data: &[u8]) -> Result<(), ParseError> { Ok(()) }
    async fn on_cmd(&mut self, _ctx: &Context, _ch: &CmdHeader, _d: &[u8]) -> Result<(), ParseError> { Ok(()) }
    async fn on_tick_end(&mut self, _ctx: &Context) -> Result<(), ParseError> { Ok(()) }

    async fn on_entity(
        &mut self,
        ctx: &Context,
        delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<(), ParseError> {
        if self.printed >= 10 { return Ok(()); }

        // Track game start time from game rules
        if entity.serializer().serializer_name.hash == GAMERULES {
            if let Some(t) = entity.get_value::<f32>(&GAME_START_KEY) {
                if t > 0.0 && self.game_start_time.is_none() {
                    self.game_start_time = Some(t);
                }
            }
        }

        // Pick the first CCitadelPlayerPawn we see and track it
        if entity.serializer().serializer_name.hash == CCITADELPLAYERPAWN {
            if self.tracked_entity_index.is_none() && delta_header == DeltaHeader::CREATE {
                self.tracked_entity_index = Some(entity.index());
                println!("Tracking CCitadelPlayerPawn entity index={}", entity.index());
                println!("{:<6}  {:>12}  {:>12}  {:>10}  {:>10}  {:>10}  {:>10}",
                    "sec", "old_cellX", "old_vecX", "new_cellX", "new_vecX", "world_x", "world_y");
                println!("{}", "-".repeat(78));
            }

            if Some(entity.index()) != self.tracked_entity_index { return Ok(()); }
            if delta_header != DeltaHeader::UPDATE { return Ok(()); }

            let Some(start) = self.game_start_time else { return Ok(()); };
            let current_time = (ctx.tick() as f32) * ctx.tick_interval();
            let match_second = (current_time - start) as i32;

            if match_second < 0 || match_second == self.last_printed_second { return Ok(()); }
            self.last_printed_second = match_second;
            self.printed += 1;

            // Old path -- should always be None
            let old_cx: Option<u64> = entity.get_value(&OLD_CX);
            let old_vx: Option<f32> = entity.get_value(&OLD_VX);
            let _ = entity.get_value::<u64>(&OLD_CY);
            let _ = entity.get_value::<f32>(&OLD_VY);

            // New path -- should have values
            let new_cx: Option<u64> = entity.get_value(&NEW_CX);
            let new_cy: Option<u64> = entity.get_value(&NEW_CY);
            let new_vx: Option<f32> = entity.get_value(&NEW_VX);
            let new_vy: Option<f32> = entity.get_value(&NEW_VY);

            let world_x = match (new_cx, new_vx) {
                (Some(cx), Some(vx)) => format!("{:.1}", deadlock_coord_from_cell(cx as u16, vx)),
                _ => "N/A".to_string(),
            };
            let world_y = match (new_cy, new_vy) {
                (Some(cy), Some(vy)) => format!("{:.1}", deadlock_coord_from_cell(cy as u16, vy)),
                _ => "N/A".to_string(),
            };

            println!("{:<6}  {:>12}  {:>12}  {:>10}  {:>10}  {:>10}  {:>10}",
                match_second,
                old_cx.map(|v| v.to_string()).unwrap_or("None".to_string()),
                old_vx.map(|v| format!("{:.3}", v)).unwrap_or("None".to_string()),
                new_cx.map(|v| v.to_string()).unwrap_or("None".to_string()),
                new_vx.map(|v| format!("{:.3}", v)).unwrap_or("None".to_string()),
                world_x,
                world_y,
            );
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args().nth(1).expect("Usage: probe_positions <path-to.dem>");
    println!("Probing: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut visitor = ProbeVisitor {
        game_start_time: None,
        tracked_entity_index: None,
        last_printed_second: -1,
        printed: 0,
    };
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    Ok(())
}
