//! Creep wave tracking for lane pressure analysis

use std::collections::HashMap;

use haste::entities::Entity;
use tracing::{debug, info};

use crate::domain::{CreepWaveData, CreepWaveSnapshot};
use crate::entities::constants::{CNPC_TROOPER_ENTITY, TEAM_KEY};
use crate::utils::get_entity_position;

/// Key for grouping creeps: (lane, team)
type WaveKey = (i32, u32);

/// Distance threshold for clustering creeps into separate waves (in world units).
/// Creeps within this distance of a wave centroid are considered part of that wave.
/// Waves spawn every 30 seconds with 4 creeps. This threshold should be large enough
/// to group a single wave but small enough to keep separate waves (and straggler creeps)
/// distinct.
const WAVE_CLUSTER_THRESHOLD: f32 = 1000.0;

/// Active creep entity data
#[derive(Debug, Clone)]
struct ActiveCreep {
    entity_index: i32,
    lane: i32,
    team: u32,
    x: f32,
    y: f32,
}

/// Tracks lane creep (trooper) entities to compute wave positions
#[derive(Debug)]
pub struct CreepTracker {
    /// Field key for lane
    lane_key: u64,

    /// Currently active creeps by entity index
    active_creeps: HashMap<i32, ActiveCreep>,

    /// Per-second wave snapshots grouped by lane
    /// Key: lane as string, Value: Vec of per-second snapshots
    wave_timeline: HashMap<String, Vec<Option<CreepWaveSnapshot>>>,

    /// Track which second we're on
    current_window: u32,
}

impl CreepTracker {
    pub fn new(lane_key: u64) -> Self {
        Self {
            lane_key,
            active_creeps: HashMap::new(),
            wave_timeline: HashMap::new(),
            current_window: 0,
        }
    }

    /// Check if an entity is a lane creep (trooper)
    pub fn is_creep_entity(hash: u64) -> bool {
        hash == CNPC_TROOPER_ENTITY
    }

    /// Handle creep entity creation
    pub fn handle_creep_create(&mut self, entity: &Entity) {
        let position = get_entity_position(entity);
        let team: u32 = entity.get_value(&TEAM_KEY).unwrap_or(0);
        let lane: i32 = entity.get_value(&self.lane_key).unwrap_or(0);

        debug!(
            "[creep_tracker] Creep created: entity={}, lane={}, team={}, pos=({:.0}, {:.0})",
            entity.index(),
            lane,
            team,
            position[0],
            position[1]
        );

        let creep = ActiveCreep {
            entity_index: entity.index(),
            lane,
            team,
            x: position[0],
            y: position[1],
        };

        self.active_creeps.insert(entity.index(), creep);
    }

    /// Handle creep entity deletion (death)
    pub fn handle_creep_delete(&mut self, entity_index: i32) {
        self.active_creeps.remove(&entity_index);
    }

    /// Handle creep position update - also adds creeps not yet tracked
    pub fn handle_creep_update(&mut self, entity: &Entity) {
        let position = get_entity_position(entity);

        if let Some(creep) = self.active_creeps.get_mut(&entity.index()) {
            // Update existing creep position
            creep.x = position[0];
            creep.y = position[1];
        } else {
            // Add pre-existing creep that wasn't tracked yet
            let team: u32 = entity.get_value(&TEAM_KEY).unwrap_or(0);
            let lane: i32 = entity.get_value(&self.lane_key).unwrap_or(0);

            let creep = ActiveCreep {
                entity_index: entity.index(),
                lane,
                team,
                x: position[0],
                y: position[1],
            };

            self.active_creeps.insert(entity.index(), creep);
        }
    }

    /// Build wave snapshots for the current time window.
    /// Groups creeps by (lane, team), then spatially clusters within each group
    /// to keep separate waves distinct until they merge.
    pub fn build_wave_window(&mut self, window_s: u32) {
        // Fill in any missing windows with None
        while self.current_window < window_s {
            for timeline in self.wave_timeline.values_mut() {
                timeline.push(None);
            }
            self.current_window += 1;
        }

        // Group active creeps by (lane, team)
        let mut wave_groups: HashMap<WaveKey, Vec<&ActiveCreep>> = HashMap::new();

        for creep in self.active_creeps.values() {
            let key = (creep.lane, creep.team);
            wave_groups.entry(key).or_default().push(creep);
        }

        // Process each lane/team group with spatial clustering
        for ((lane, team), creeps) in wave_groups {
            if creeps.is_empty() {
                continue;
            }

            // Cluster creeps spatially within this lane/team
            let clusters = Self::cluster_creeps(&creeps);

            // Create a snapshot for each cluster (separate wave)
            for (wave_idx, cluster) in clusters.iter().enumerate() {
                let count = cluster.len() as u32;
                let (sum_x, sum_y) = cluster
                    .iter()
                    .fold((0.0, 0.0), |(sx, sy), c| (sx + c.x, sy + c.y));

                let snapshot = CreepWaveSnapshot {
                    x: sum_x / count as f32,
                    y: sum_y / count as f32,
                    count,
                    team,
                };

                // Use lane_team_waveIdx as key (e.g., "1_2_0" for lane 1, team 2, wave 0)
                let lane_key = format!("{}_{}_{}", lane, team, wave_idx);
                let timeline = self.wave_timeline.entry(lane_key).or_default();

                // Ensure timeline is long enough
                while timeline.len() < window_s as usize {
                    timeline.push(None);
                }

                // Add snapshot for this window
                if timeline.len() == window_s as usize {
                    timeline.push(Some(snapshot));
                } else {
                    timeline[window_s as usize] = Some(snapshot);
                }
            }
        }

        self.current_window = window_s + 1;
    }

    /// Cluster creeps spatially using proximity threshold.
    /// Returns a vector of clusters, where each cluster is a vector of creeps.
    fn cluster_creeps<'a>(creeps: &[&'a ActiveCreep]) -> Vec<Vec<&'a ActiveCreep>> {
        if creeps.is_empty() {
            return vec![];
        }

        // Sort creeps by Y position (waves move along Y axis)
        let mut sorted_creeps: Vec<&ActiveCreep> = creeps.to_vec();
        sorted_creeps.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

        let mut clusters: Vec<Vec<&'a ActiveCreep>> = vec![];

        for creep in sorted_creeps {
            // Find nearest cluster within threshold
            let mut best_cluster_idx: Option<usize> = None;
            let mut best_distance = f32::MAX;

            for (idx, cluster) in clusters.iter().enumerate() {
                // Calculate cluster centroid
                let count = cluster.len() as f32;
                let (sum_x, sum_y) = cluster
                    .iter()
                    .fold((0.0, 0.0), |(sx, sy), c| (sx + c.x, sy + c.y));
                let centroid_x = sum_x / count;
                let centroid_y = sum_y / count;

                // Calculate distance to this cluster's centroid
                let dx = creep.x - centroid_x;
                let dy = creep.y - centroid_y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < WAVE_CLUSTER_THRESHOLD && distance < best_distance {
                    best_cluster_idx = Some(idx);
                    best_distance = distance;
                }
            }

            match best_cluster_idx {
                Some(idx) => {
                    // Add to existing cluster
                    clusters[idx].push(creep);
                }
                None => {
                    // Create new cluster
                    clusters.push(vec![creep]);
                }
            }
        }

        // Sort clusters by their centroid Y position for consistent ordering
        clusters.sort_by(|a, b| {
            let a_y: f32 = a.iter().map(|c| c.y).sum::<f32>() / a.len() as f32;
            let b_y: f32 = b.iter().map(|c| c.y).sum::<f32>() / b.len() as f32;
            a_y.partial_cmp(&b_y).unwrap_or(std::cmp::Ordering::Equal)
        });

        clusters
    }

/// Get creep wave data for JSON serialization
    pub fn get_output(&self) -> CreepWaveData {
        let total_waves: usize = self.wave_timeline.values().map(|v| v.len()).sum();
        let non_null_waves: usize = self
            .wave_timeline
            .values()
            .flat_map(|v| v.iter())
            .filter(|s| s.is_some())
            .count();
        info!(
            "[creep_tracker] Output: {} lane/team combos, {} total windows, {} non-null snapshots",
            self.wave_timeline.len(),
            total_waves,
            non_null_waves
        );
        CreepWaveData {
            waves: self.wave_timeline.clone(),
        }
    }
}
