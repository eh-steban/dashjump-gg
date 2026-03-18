"""Lane pressure calculation service.

Computes per-wave, per-second pressure based on alive creep positions relative
to the nearest alive enemy lane objective (lane-assigned, not proximity-based).

Formula per second:
  centroid      = mean position of alive creeps in wave
  target        = nearest alive enemy objective (Guardian → Walker → ... → Patron)
  own_frontline = nearest alive own objective (same priority ordering, own team)
  lane_length   = euclidean(own_frontline, target)  -- dynamic; shrinks as objectives die
  dist          = euclidean(centroid, target)
  raw_pressure  = clamp(1.0 - dist / lane_length, 0.0, 1.0)
  pressure      = raw_pressure * (alive_creep_count * 0.25)
"""

import math
from typing import Optional

from app.domain.boss import BossData, BossSnapshot
from app.domain.creep import LaneCreepData, CreepSnapshot
from app.domain.lane_pressure import LanePressureData, LanePressureSnapshot
from app.utils.logger import get_logger

logger = get_logger(__name__)

# Boss type priority (lower number = attacked first by creeps)
# Maps boss_name_hash custom_id values from parser to priority order.
# Priority: Guardian=1, Walker=2, Base Guardian=3, Shrine=4, Patron=5
_BOSS_PRIORITY: dict[int, int] = {
    21: 1,  # CNPC_TROOPERBOSS_ENTITY  - Guardian
    25: 2,  # CNPC_BOSS_TIER2_ENTITY   - Walker
    26: 3,  # CNPC_BARRACKBOSS_ENTITY  - Base Guardian
    27: 4,  # CCITADEL_DESTROYABLE_BUILDING_ENTITY - Shrine
    28: 5,  # CNPC_BOSS_TIER3_ENTITY   - Patron
}

# Enemy team mapping
# bidirectional lookup table:
# team 2's enemy is team 3, and team 3's enemy is team 2
_ENEMY_TEAM: dict[int, int] = {2: 3, 3: 2}


def _euclidean(x1: float, y1: float, x2: float, y2: float) -> float:
    dx = x1 - x2
    dy = y1 - y2
    return math.sqrt(dx * dx + dy * dy)


def _clamp(value: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, value))


class LanePressureCalculator:
    """Calculate lane pressure from per-creep data and boss objective positions."""

    # ---------------------------------------------------------------------------
    # Public entry point
    # ---------------------------------------------------------------------------

    @staticmethod
    def process_creep_waves(
        lane_creep_data: LaneCreepData,
        boss_data: BossData,
    ) -> LanePressureData:
        """Process all waves and calculate lane pressure per second.

        Args:
            lane_creep_data: Per-creep timeline data from parser.
            boss_data: Boss snapshot and health timeline data.

        Returns:
            LanePressureData keyed by wave_id.
        """
        if not lane_creep_data or not lane_creep_data.wave_meta:
            logger.warning("No lane creep data available - returning empty pressure data")
            return LanePressureData(pressure={})

        # Build objective lookup: (lane, team) -> sorted list of BossSnapshots by priority
        objective_map = LanePressureCalculator._build_objective_map(boss_data)

        pressure_timeline: dict[str, list[Optional[LanePressureSnapshot]]] = {}

        for wave_id, wave_meta in lane_creep_data.wave_meta.items():
            lane = wave_meta.lane
            team = wave_meta.team
            enemy_team = _ENEMY_TEAM.get(team)
            if enemy_team is None:
                logger.error("Unknown team id %d in wave %s -- skipping wave", team, wave_id)
                continue

            own_objectives = objective_map.get((lane, team), [])
            enemy_objectives = objective_map.get((lane, enemy_team), [])

            # Determine timeline length from creep data
            timeline_length = LanePressureCalculator._wave_timeline_length(
                wave_id, lane_creep_data
            )
            if timeline_length == 0:
                continue

            snapshots: list[Optional[LanePressureSnapshot]] = []

            for second in range(timeline_length):
                alive_creeps = LanePressureCalculator._alive_creeps_at(
                    wave_id, second, lane_creep_data
                )

                if not alive_creeps:
                    snapshots.append(None)
                    continue

                # Centroid (internal only — not in output)
                centroid_x = sum(c.x for c in alive_creeps) / len(alive_creeps)
                centroid_y = sum(c.y for c in alive_creeps) / len(alive_creeps)

                # Dynamic lane_length: distance between own frontline and enemy frontline at this second
                target = LanePressureCalculator._current_target(
                    enemy_objectives, boss_data, second
                )
                own_frontline = LanePressureCalculator._own_frontline_objective(
                    own_objectives, boss_data, second
                )
                lane_length = _euclidean(own_frontline.x, own_frontline.y, target.x, target.y)
                dist = _euclidean(centroid_x, centroid_y, target.x, target.y)
                raw_pressure = _clamp(1.0 - dist / lane_length, 0.0, 1.0)

                pressure = raw_pressure * (len(alive_creeps) * 0.25)

                attributed_players: list[int] = []
                seen: set[int] = set()
                for creep in alive_creeps:
                    for pid in creep.nearby_players:
                        if pid not in seen:
                            seen.add(pid)
                            attributed_players.append(pid)

                snapshots.append(
                    LanePressureSnapshot(
                        pressure=pressure,
                        team=team,
                        wave_id=wave_id,
                        creep_count=len(alive_creeps),
                        attributed_players=attributed_players,
                    )
                )

            pressure_timeline[wave_id] = snapshots

        logger.debug(
            "Calculated pressure for %d wave(s)",
            len(pressure_timeline),
        )

        return LanePressureData(pressure=pressure_timeline)

    # ---------------------------------------------------------------------------
    # Internal helpers
    # ---------------------------------------------------------------------------

    @staticmethod
    def _build_objective_map(
        boss_data: BossData,
    ) -> dict[tuple[int, int], list[BossSnapshot]]:
        """Build lookup: (lane, team) -> BossSnapshots sorted by attack priority."""
        objective_map: dict[tuple[int, int], list[BossSnapshot]] = {}

        for snap in boss_data.snapshots:
            key = (snap.lane, snap.team)
            objective_map.setdefault(key, []).append(snap)

        # Sort each bucket by priority (lower custom_id priority value = first target)
        for key in objective_map:
            objective_map[key].sort(
                key=lambda s: _BOSS_PRIORITY.get(s.custom_id, 99)
            )

        return objective_map

    @staticmethod
    def _wave_timeline_length(wave_id: str, lane_creep_data: LaneCreepData) -> int:
        """Return the timeline length for this wave by finding any live creep snapshot.

        Every wave has at least one live snapshot (creeps live for at least ~1 second),
        so we can return as soon as we find a matching entry.
        """
        for timeline in lane_creep_data.creeps.values():
            for snap in timeline:
                if snap is not None and snap.wave_id == wave_id:
                    return len(timeline)
        logger.warning(
            "No live snapshots found for wave %s -- wave_meta references unknown wave (orphan wave; skipping)",
            wave_id,
        )
        return 0

    @staticmethod
    def _alive_creeps_at(
        wave_id: str,
        second: int,
        lane_creep_data: LaneCreepData,
    ) -> list[CreepSnapshot]:
        """Return all alive creep snapshots for a wave at a given second."""
        alive: list[CreepSnapshot] = []
        for timeline in lane_creep_data.creeps.values():
            if second >= len(timeline):
                continue
            snap = timeline[second]
            if snap is not None and snap.wave_id == wave_id:
                alive.append(snap)
        return alive

    @staticmethod
    def _current_target(
        enemy_objectives: list[BossSnapshot],
        boss_data: BossData,
        second: int,
    ) -> BossSnapshot:
        """Find the lowest-priority enemy objective that is alive at the given second.

        Uses health_timeline to determine whether a boss is alive.
        health_timeline is a list of dicts keyed by str(entity_index).
        """
        if second >= len(boss_data.health_timeline):
            # Past the end of recorded data — treat first objective as alive
            return enemy_objectives[0]

        health_window = boss_data.health_timeline[second]

        for boss in enemy_objectives:  # Already sorted by priority
            key = str(boss.entity_index)
            health = health_window.get(key)
            if health is None or health > 0:
                # Either health not recorded (treat as alive) or explicitly > 0
                return boss

        # All objectives have recorded health == 0; game should be over at this point.
        # Return the patron as a safe fallback.
        return enemy_objectives[-1]

    @staticmethod
    def _own_frontline_objective(
        own_objectives: list[BossSnapshot],
        boss_data: BossData,
        second: int,
    ) -> BossSnapshot:
        """Find the lowest-priority own objective still alive at this second.

        This is the 'own frontline' -- the nearest own objective to neutral ground.
        Used as the near endpoint when computing dynamic lane_length per second.
        """
        if second >= len(boss_data.health_timeline):
            return own_objectives[0]

        health_window = boss_data.health_timeline[second]

        for boss in own_objectives:  # Already sorted by priority
            key = str(boss.entity_index)
            health = health_window.get(key)
            if health is None or health > 0:
                return boss

        # All own objectives have recorded health == 0; game should be over.
        return own_objectives[-1]
