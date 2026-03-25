/// Minimal CLI tool: parse a local .dem file and confirm PostMatch decode.
/// Usage: cargo run --bin parse_local -- <path-to.dem>
///
/// Prints "PostMatch damage_matrix: dealers=N samples=M" if CMsgMatchMetaDataContentsPatched
/// decoded successfully, confirming the haste migration works end-to-end.
use std::env;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use haste::demofile::DemoFile;
use haste::demostream::CmdHeader;
use haste::entities::{DeltaHeader, Entity};
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::deadlock::{
    CCitadelUserMsgPostMatchDetails,
    CitadelUserMessageIds,
    CMsgMatchMetaDataContentsPatched,
};
use prost::Message;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct ParseError(#[from] anyhow::Error);

impl From<prost::DecodeError> for ParseError {
    fn from(e: prost::DecodeError) -> Self {
        ParseError(anyhow::Error::from(e))
    }
}

struct ProbeVisitor {
    postmatch_found: bool,
    dealers: usize,
    samples: usize,
}

impl Default for ProbeVisitor {
    fn default() -> Self {
        Self {
            postmatch_found: false,
            dealers: 0,
            samples: 0,
        }
    }
}

impl Visitor for &mut ProbeVisitor {
    type Error = ParseError;

    async fn on_packet(
        &mut self,
        _ctx: &Context,
        packet_type: u32,
        data: &[u8],
    ) -> Result<(), ParseError> {
        if packet_type == CitadelUserMessageIds::KEUserMsgPostMatchDetails as u32 {
            let details_msg = CCitadelUserMsgPostMatchDetails::decode(data)?;
            if let Some(bytes) = details_msg.match_details {
                let mut cursor = std::io::Cursor::new(bytes);
                if let Ok(meta) = CMsgMatchMetaDataContentsPatched::decode(&mut cursor) {
                    if let Some(match_info) = meta.match_info {
                        if let Some(damage_matrix) = match_info.damage_matrix {
                            self.dealers = damage_matrix.damage_dealers.len();
                            self.samples = damage_matrix.sample_time_s.len();
                            self.postmatch_found = true;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn on_entity(
        &mut self,
        _ctx: &Context,
        _delta_header: DeltaHeader,
        _entity: &Entity,
    ) -> Result<(), ParseError> {
        Ok(())
    }

    async fn on_cmd(
        &mut self,
        _ctx: &Context,
        _cmd_header: &CmdHeader,
        _data: &[u8],
    ) -> Result<(), ParseError> {
        Ok(())
    }

    async fn on_tick_end(&mut self, _ctx: &Context) -> Result<(), ParseError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let path = env::args().nth(1).expect("Usage: parse_local <path-to.dem>");
    println!("Parsing: {}", path);

    let file = File::open(&path)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut visitor = ProbeVisitor::default();
    let mut parser = Parser::from_stream_with_visitor(demo_file, &mut visitor)?;
    parser.run_to_end().await?;

    if visitor.postmatch_found {
        println!(
            "PostMatch damage_matrix: dealers={} samples={}",
            visitor.dealers, visitor.samples
        );
    } else {
        println!("PostMatch: NOT FOUND -- CMsgMatchMetaDataContentsPatched did not decode");
    }

    Ok(())
}
