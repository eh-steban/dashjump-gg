"""Lane pressure calculation service.

Computes per-wave, per-second pressure by projecting the creep centroid onto
the static waypoint chain that defines each lane's physical path. Lanes 1 and 6
bend sharply near the base (guardian is horizontal midfield; base guardian is
vertical near the base), so straight-line Euclidean distance from centroid to
target understates progress when creeps sit in the bend. Path projection along
the waypoint chain corrects that.

Formula per second:
  centroid           = mean position of alive creeps in wave
  target             = nearest alive enemy objective (Guardian -> Walker -> ... -> Patron)
  own_frontline      = nearest alive own objective (same priority ordering, own team)
  lane_path          = static polyline
                       [own_patron, own_base_guard_midpoint, own_walker, own_guardian,
                        enemy_guardian, enemy_walker, enemy_base_guard_midpoint, enemy_patron]
  centroid_progress  = cumulative distance from own_patron to the nearest projection
                       of centroid onto the polyline
  own_frontline_prog = same projection applied to own_frontline
  target_prog        = same projection applied to target
  contested_zone_len = target_prog - own_frontline_prog
  raw_pressure       = clamp((centroid_progress - own_frontline_prog) / contested_zone_len, 0, 1)
  pressure           = raw_pressure * (alive_creep_count * 0.25)

When the lane path cannot be built (degenerate boss data -- e.g. tests with only
two guardians), the calculation falls back to the original
`1 - euclidean(centroid, target) / euclidean(own_frontline, target)` formula. Both
formulas agree on straight lanes; the path variant is only meaningfully different
on lanes with a bend.

Ghost creep filtering note:
  The parser's CreepTracker already suppresses dead and ghost entities at the snapshot
  layer using an NPC state whitelist (IDLE | ALERT | COMBAT | INERT) combined with
  life_state == LIFE_ALIVE. A None entry in a creep timeline means the entity was
  dead, dying, or not yet active at that second. No additional ghost filtering is
  needed here.
"""

import math
from typing import Optional

from app.domain.boss import BossData, BossSnapshot
from app.domain.creep import LaneCreepData, CreepSnapshot
from app.domain.lane_pressure import LanePressureData, LanePressureSnapshot
from app.utils.logger import get_logger

logger = get_logger(__name__)

# Canonical boss type identifiers.
#
# These are the u64 fxhash values of the SendTable serializer names, computed
# in parser/src/entities/constants.rs at Rust compile time and emitted on every
# BossSnapshot as `boss_name_hash`. They are the single source of truth for
# boss-type matching across services -- `custom_id` on BossSnapshot is a
# convenience enum from the parser's `get_custom_id` and is not load-bearing
# here.
#
# To resync if parser/src/entities/constants.rs adds or renames a boss type:
#   DELETE FROM parsedmatch WHERE match_id = <any parsed match>;
#   curl http://localhost:8000/match/analysis/<that match>;
#   SELECT DISTINCT (s->>'boss_name_hash') FROM parsedmatch,
#     jsonb_array_elements(match_data->'bosses'->'snapshots') s
#     WHERE match_id = <that match>;
# and paste the values here.
BOSS_HASH_GUARDIAN = 12946736302082733589       # CNPC_TrooperBoss
BOSS_HASH_WALKER = 1942975293714691302          # CNPC_Boss_Tier2
BOSS_HASH_BASE_GUARDIAN = 793562361056549792    # CNPC_BarrackBoss
BOSS_HASH_SHRINE = 8292725763874089450          # CCitadel_Destroyable_Building
BOSS_HASH_PATRON = 7814756300278693755          # CNPC_Boss_Tier3

# Attack priority (lower number = attacked first by creeps).
# Keyed on boss_name_hash so the mapping stays anchored to constants.rs.
_BOSS_PRIORITY: dict[int, int] = {
    BOSS_HASH_GUARDIAN: 1,
    BOSS_HASH_WALKER: 2,
    BOSS_HASH_BASE_GUARDIAN: 3,
    BOSS_HASH_SHRINE: 4,
    BOSS_HASH_PATRON: 5,
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


def _project_onto_path(
    px: float,
    py: float,
    waypoints: list[tuple[float, float]],
) -> tuple[float, float]:
    """Project a point onto a polyline.

    Returns (progress, total_length) where progress is the cumulative distance
    from waypoints[0] along the path to the nearest projection point, and
    total_length is the full polyline length. If waypoints has fewer than two
    points, both values are 0.0.
    """
    if len(waypoints) < 2:
        return 0.0, 0.0

    best_progress = 0.0
    best_dist_sq = float("inf")
    cumulative = 0.0

    for i in range(len(waypoints) - 1):
        ax, ay = waypoints[i]
        bx, by = waypoints[i + 1]
        seg_dx = bx - ax
        seg_dy = by - ay
        seg_len_sq = seg_dx * seg_dx + seg_dy * seg_dy
        if seg_len_sq == 0.0:
            continue
        seg_len = math.sqrt(seg_len_sq)

        # Project (px, py) onto the segment [a, b], clamped to [0, 1]
        t = ((px - ax) * seg_dx + (py - ay) * seg_dy) / seg_len_sq
        t_clamped = _clamp(t, 0.0, 1.0)

        proj_x = ax + t_clamped * seg_dx
        proj_y = ay + t_clamped * seg_dy
        dx = px - proj_x
        dy = py - proj_y
        dist_sq = dx * dx + dy * dy

        if dist_sq < best_dist_sq:
            best_dist_sq = dist_sq
            best_progress = cumulative + t_clamped * seg_len

        cumulative += seg_len

    return best_progress, cumulative


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

        # Static waypoint chains per (lane, team) for path-aware pressure calc.
        lane_paths = LanePressureCalculator._build_lane_paths(boss_data)

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
            lane_path = lane_paths.get((lane, team), [])

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

                # Centroid (internal only -- not in output)
                centroid_x = sum(c.x for c in alive_creeps) / len(alive_creeps)
                centroid_y = sum(c.y for c in alive_creeps) / len(alive_creeps)

                target = LanePressureCalculator._current_target(
                    enemy_objectives, boss_data, second
                )
                own_frontline = LanePressureCalculator._own_frontline_objective(
                    own_objectives, boss_data, second
                )

                raw_pressure = LanePressureCalculator._raw_pressure(
                    centroid_x=centroid_x,
                    centroid_y=centroid_y,
                    own_frontline=own_frontline,
                    target=target,
                    lane_path=lane_path,
                )

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
    def _raw_pressure(
        centroid_x: float,
        centroid_y: float,
        own_frontline: BossSnapshot,
        target: BossSnapshot,
        lane_path: list[tuple[float, float]],
    ) -> float:
        """Compute raw pressure in [0, 1] for a centroid against a contested zone.

        When `lane_path` has at least two waypoints, projects the centroid,
        own frontline, and target onto the polyline and computes progress
        within the contested zone. Otherwise falls back to the Euclidean
        formula (used by degenerate test fixtures and any lanes where the
        path could not be built).
        """
        if len(lane_path) >= 2:
            centroid_prog, _ = _project_onto_path(centroid_x, centroid_y, lane_path)
            own_prog, _ = _project_onto_path(own_frontline.x, own_frontline.y, lane_path)
            target_prog, _ = _project_onto_path(target.x, target.y, lane_path)
            zone_len = target_prog - own_prog
            if zone_len > 0:
                return _clamp((centroid_prog - own_prog) / zone_len, 0.0, 1.0)
            # Degenerate: own_frontline projects past the target. Fall through
            # to Euclidean rather than returning 0 so unit tests with a single
            # own objective at/beyond the target still produce sensible values.

        lane_length = _euclidean(own_frontline.x, own_frontline.y, target.x, target.y)
        if lane_length == 0.0:
            return 0.0
        dist = _euclidean(centroid_x, centroid_y, target.x, target.y)
        return _clamp(1.0 - dist / lane_length, 0.0, 1.0)

    @staticmethod
    def _build_lane_paths(
        boss_data: BossData,
    ) -> dict[tuple[int, int], list[tuple[float, float]]]:
        """Build the static waypoint chain for each (lane, team) combination.

        Waypoint order from the perspective of `team`:
            own_patron -> own_base_guardian_midpoint -> own_walker -> own_guardian
            -> enemy_guardian -> enemy_walker -> enemy_base_guardian_midpoint -> enemy_patron

        Patrons sit at lane=0 (shared, not per-lane) and anchor both ends.
        Base guardians come in pairs per lane per team; the pair is collapsed
        to its midpoint to avoid double-counting.

        Missing objectives are skipped rather than substituted. A path with
        fewer than two surviving waypoints falls back to Euclidean at the
        call site.
        """
        by_key: dict[tuple[int, int, int], list[BossSnapshot]] = {}
        for s in boss_data.snapshots:
            by_key.setdefault((s.team, s.lane, s.boss_name_hash), []).append(s)

        def single(team: int, lane: int, hash_: int) -> Optional[tuple[float, float]]:
            matches = by_key.get((team, lane, hash_), [])
            if not matches:
                return None
            return (matches[0].x, matches[0].y)

        def midpoint(team: int, lane: int, hash_: int) -> Optional[tuple[float, float]]:
            matches = by_key.get((team, lane, hash_), [])
            if not matches:
                return None
            mean_x = sum(s.x for s in matches) / len(matches)
            mean_y = sum(s.y for s in matches) / len(matches)
            return (mean_x, mean_y)

        paths: dict[tuple[int, int], list[tuple[float, float]]] = {}

        # Lanes to build paths for: anything that has per-lane objectives.
        lane_ids = {s.lane for s in boss_data.snapshots if s.lane != 0}

        for team in (2, 3):
            enemy = _ENEMY_TEAM[team]
            own_patron = single(team, 0, BOSS_HASH_PATRON)
            enemy_patron = single(enemy, 0, BOSS_HASH_PATRON)

            for lane in lane_ids:
                wp: list[tuple[float, float]] = []
                if own_patron:
                    wp.append(own_patron)
                own_bg = midpoint(team, lane, BOSS_HASH_BASE_GUARDIAN)
                if own_bg:
                    wp.append(own_bg)
                own_walker = single(team, lane, BOSS_HASH_WALKER)
                if own_walker:
                    wp.append(own_walker)
                own_guardian = single(team, lane, BOSS_HASH_GUARDIAN)
                if own_guardian:
                    wp.append(own_guardian)

                enemy_guardian = single(enemy, lane, BOSS_HASH_GUARDIAN)
                if enemy_guardian:
                    wp.append(enemy_guardian)
                enemy_walker = single(enemy, lane, BOSS_HASH_WALKER)
                if enemy_walker:
                    wp.append(enemy_walker)
                enemy_bg = midpoint(enemy, lane, BOSS_HASH_BASE_GUARDIAN)
                if enemy_bg:
                    wp.append(enemy_bg)
                if enemy_patron:
                    wp.append(enemy_patron)

                if len(wp) >= 2:
                    paths[(lane, team)] = wp

        return paths

    @staticmethod
    def _build_objective_map(
        boss_data: BossData,
    ) -> dict[tuple[int, int], list[BossSnapshot]]:
        """Build lookup: (lane, team) -> BossSnapshots sorted by attack priority."""
        objective_map: dict[tuple[int, int], list[BossSnapshot]] = {}

        for snap in boss_data.snapshots:
            key = (snap.lane, snap.team)
            objective_map.setdefault(key, []).append(snap)

        # Sort each bucket by attack priority. Priority is keyed on boss_name_hash
        # (u64 fxhash from parser/src/entities/constants.rs), not custom_id.
        for key in objective_map:
            objective_map[key].sort(
                key=lambda s: _BOSS_PRIORITY.get(s.boss_name_hash, 99)
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
        """Return all alive creep snapshots for a wave at a given second.

        A non-None snapshot is alive by definition -- the parser's CreepTracker
        suppresses dead and ghost entities at the source using an NPC state whitelist.
        Cage creeps (is_cage=True) are excluded: they are zipline transit units,
        not lane fighters.
        """
        alive: list[CreepSnapshot] = []
        for timeline in lane_creep_data.creeps.values():
            if second >= len(timeline):
                continue
            snap = timeline[second]
            if snap is not None and snap.wave_id == wave_id and not snap.is_cage:
                alive.append(snap)
        return alive

    @staticmethod
    def _near_objective(
        snap: CreepSnapshot,
        objective_positions: list[tuple[float, float]],
        range_units: float,
    ) -> bool:
        """Return True if this snapshot is within range_units of any objective position.

        Utility for objective-proximity analysis -- e.g., attributing players or creeps
        to an objective fight.
        """
        for ox, oy in objective_positions:
            dx = snap.x - ox
            dy = snap.y - oy
            if dx * dx + dy * dy < range_units * range_units:
                return True
        return False

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
