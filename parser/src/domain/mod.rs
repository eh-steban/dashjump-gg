//! Domain models for parsed replay data

pub mod boss;
pub mod creep;
pub mod damage;
pub mod player;
pub mod sinner;

pub use boss::BossSnapshot;
pub use creep::{CreepSnapshot, CreepTimeline, LaneCreepData, WaveMeta};
pub use damage::DamageRecord;
pub use player::{Player, PlayerPosition};
pub use sinner::{SinnerDamageEvent, SinnerDamageKind, SinnerSnapshot};
