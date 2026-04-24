"""Unit and integration tests for LanePressureCalculator.

All fixtures include the full objective chain (Guardian -> Walker -> Base Guardian
-> Shrine -> Patron) for both teams so that `_build_lane_paths` always produces a
usable waypoint polyline. A fixture with fewer objectives would land in the
degraded-path branch of `_raw_pressure`, which logs an ERROR and returns None.
Tests that exercise degradation explicitly opt into that branch.
"""

import logging
from typing import Optional

import pytest

from app.domain.boss import BossData, BossSnapshot
from app.domain.creep import CreepSnapshot, LaneCreepData, WaveMeta
from app.domain.lane_pressure import LanePressureData, LanePressureSnapshot
from app.services.lane_pressure_service import (
    BOSS_HASH_BASE_GUARDIAN,
    BOSS_HASH_GUARDIAN,
    BOSS_HASH_PATRON,
    BOSS_HASH_SHRINE,
    BOSS_HASH_WALKER,
    LanePressureCalculator,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

def _make_boss(
    entity_index: int,
    hash_: str,
    lane: int,
    team: int,
    x: float,
    y: float,
) -> BossSnapshot:
    """Build a BossSnapshot mock. `hash_` is the boss_name_hash decimal string
    (u64 transported as string -- see parser-api.md). Use one of the
    BOSS_HASH_* constants from lane_pressure_service to identify the boss
    type. custom_id is a convenience field that the service no longer
    consults, so it is left at 0.
    """
    return BossSnapshot(
        entity_index=entity_index,
        custom_id=0,
        boss_name_hash=hash_,
        team=team,
        lane=lane,
        x=x,
        y=y,
        z=0.0,
        spawn_time_s=0,
        max_health=10000,
        life_state_on_create=0,
    )


def _make_creep(
    lane: int,
    team: int,
    wave_id: str,
    x: float,
    y: float,
    nearby_players: Optional[list[int]] = None,
) -> CreepSnapshot:
    return CreepSnapshot(
        x=x,
        y=y,
        lane=lane,
        team=team,
        wave_id=wave_id,
        nearby_players=nearby_players or [],
    )


# Canonical straight-line fixture coordinates. All objectives share x=0 so the
# polyline collapses to a single y-axis segment chain, which makes the expected
# pressure arithmetic trivial for tests. Entity indices are deliberately stable
# so tests can drive the health_timeline by index.
#
# The y-positions match the geometric order from Deadlock (patron deepest, then
# shrine, then base guardian, walker, guardian) but spacings are round numbers.
_STRAIGHT_COORDS: dict[int, tuple[float, float]] = {
    # entity_index -> (x, y)
    1:  (0.0, -10000.0),  # own_patron          (team 2)
    2:  (0.0,  -9000.0),  # own_shrine
    3:  (0.0,  -8000.0),  # own_bg_a
    4:  (0.0,  -8000.0),  # own_bg_b (same point as bg_a -> midpoint == bg_a)
    5:  (0.0,  -6000.0),  # own_walker
    6:  (0.0,  -5000.0),  # own_guardian
    7:  (0.0,   5000.0),  # enemy_guardian      (team 3)
    8:  (0.0,   6000.0),  # enemy_walker
    9:  (0.0,   8000.0),  # enemy_bg_a
    10: (0.0,   8000.0),  # enemy_bg_b
    11: (0.0,   9000.0),  # enemy_shrine
    12: (0.0,  10000.0),  # enemy_patron
}

# Spec: (entity_index, boss_hash, team, lane)
_STRAIGHT_SPEC: list[tuple[int, int, int, int]] = [
    (1,  BOSS_HASH_PATRON,        2, 0),
    (2,  BOSS_HASH_SHRINE,        2, 0),
    (3,  BOSS_HASH_BASE_GUARDIAN, 2, 1),
    (4,  BOSS_HASH_BASE_GUARDIAN, 2, 1),
    (5,  BOSS_HASH_WALKER,        2, 1),
    (6,  BOSS_HASH_GUARDIAN,      2, 1),
    (7,  BOSS_HASH_GUARDIAN,      3, 1),
    (8,  BOSS_HASH_WALKER,        3, 1),
    (9,  BOSS_HASH_BASE_GUARDIAN, 3, 1),
    (10, BOSS_HASH_BASE_GUARDIAN, 3, 1),
    (11, BOSS_HASH_SHRINE,        3, 0),
    (12, BOSS_HASH_PATRON,        3, 0),
]


def _straight_chain(
    dead_entities: Optional[set[int]] = None,
    timeline_length: int = 1,
) -> BossData:
    """Full-chain boss data on a straight y-axis lane=1 fixture.

    `dead_entities` is a set of entity_index values whose health should read
    as 0 in the health_timeline. Omitted entities are alive.
    """
    dead = dead_entities or set()
    snapshots = [
        _make_boss(idx, hash_, lane=lane, team=team, x=_STRAIGHT_COORDS[idx][0], y=_STRAIGHT_COORDS[idx][1])
        for idx, hash_, team, lane in _STRAIGHT_SPEC
    ]
    window = {
        str(idx): (0 if idx in dead else 10000)
        for idx, *_ in _STRAIGHT_SPEC
    }
    return BossData(
        snapshots=snapshots,
        health_timeline=[dict(window) for _ in range(timeline_length)],
    )


# Convenience helpers so tests can refer to entities by name without reaching
# into _STRAIGHT_SPEC by index.
OWN_PATRON_IDX = 1
OWN_SHRINE_IDX = 2
OWN_BG_A_IDX = 3
OWN_BG_B_IDX = 4
OWN_WALKER_IDX = 5
OWN_GUARDIAN_IDX = 6
ENEMY_GUARDIAN_IDX = 7
ENEMY_WALKER_IDX = 8
ENEMY_BG_A_IDX = 9
ENEMY_BG_B_IDX = 10
ENEMY_SHRINE_IDX = 11
ENEMY_PATRON_IDX = 12


def _timeline_of_length(
    snap: Optional[CreepSnapshot],
    length: int,
) -> list[Optional[CreepSnapshot]]:
    """Create a timeline where every second has the same snapshot."""
    return [snap] * length


# ---------------------------------------------------------------------------
# Straight-lane geometry reference
# ---------------------------------------------------------------------------
#
# The polyline built from _STRAIGHT_SPEC (BG pair collapses to midpoint) is:
#
#   own_patron     y = -10000   cumulative =  0
#   own_shrine     y =  -9000   cumulative =  1000
#   own_bg_mid     y =  -8000   cumulative =  2000
#   own_walker     y =  -6000   cumulative =  4000
#   own_guardian   y =  -5000   cumulative =  5000
#   enemy_guardian y =   5000   cumulative = 15000
#   enemy_walker   y =   6000   cumulative = 16000
#   enemy_bg_mid   y =   8000   cumulative = 18000
#   enemy_shrine   y =   9000   cumulative = 19000
#   enemy_patron   y =  10000   cumulative = 20000
#
# For a centroid at (0, Y), progress along the path equals
#   5000 + (Y + 5000)      when Y is in [-5000, 5000]        (segment 4->5)
# and similar formulas for other segments. Tests compute expected values by
# projecting onto the colinear path directly.

STRAIGHT_PROG: dict[int, int] = {
    OWN_PATRON_IDX:     0,
    OWN_SHRINE_IDX:     1000,
    OWN_BG_A_IDX:       2000,   # BG midpoint
    OWN_BG_B_IDX:       2000,
    OWN_WALKER_IDX:     4000,
    OWN_GUARDIAN_IDX:   5000,
    ENEMY_GUARDIAN_IDX: 15000,
    ENEMY_WALKER_IDX:   16000,
    ENEMY_BG_A_IDX:     18000,
    ENEMY_BG_B_IDX:     18000,
    ENEMY_SHRINE_IDX:   19000,
    ENEMY_PATRON_IDX:   20000,
}


def _expected_straight_raw(
    centroid_y: float,
    own_prog: int,
    target_prog: int,
) -> float:
    """Expected raw pressure for a centroid at (0, y) on the straight fixture."""
    # Project y to progress along the straight path.
    if centroid_y <= -10000:
        c = 0
    elif centroid_y <= -8000:
        c = int(1000 * (centroid_y + 10000) / 2000 + 0)  # patron->shrine or shrine->bg
        # Walk segment by segment
        c = _straight_progress_for_y(centroid_y)
    elif centroid_y <= 10000:
        c = _straight_progress_for_y(centroid_y)
    else:
        c = 20000

    zone = target_prog - own_prog
    if zone <= 0:
        return 0.0
    return max(0.0, min(1.0, (c - own_prog) / zone))


def _straight_progress_for_y(y: float) -> float:
    """Compute cumulative path progress for a point (0, y) projected onto the
    straight-line fixture polyline."""
    waypoints = [
        (-10000.0,     0),
        ( -9000.0,  1000),
        ( -8000.0,  2000),
        ( -6000.0,  4000),
        ( -5000.0,  5000),
        (  5000.0, 15000),
        (  6000.0, 16000),
        (  8000.0, 18000),
        (  9000.0, 19000),
        ( 10000.0, 20000),
    ]
    if y <= waypoints[0][0]:
        return waypoints[0][1]
    if y >= waypoints[-1][0]:
        return waypoints[-1][1]
    for i in range(len(waypoints) - 1):
        y_a, c_a = waypoints[i]
        y_b, c_b = waypoints[i + 1]
        if y_a <= y <= y_b:
            t = (y - y_a) / (y_b - y_a)
            return c_a + t * (c_b - c_a)
    return waypoints[-1][1]


# ---------------------------------------------------------------------------
# 1)  Four alive creeps in contested zone -> pressure between 0.25 and 1.0
# ---------------------------------------------------------------------------

class TestPressureWithAliveCreeps:
    def test_pressure_in_valid_range(self):
        """Four alive creeps close to enemy guardian should produce pressure between 0.25 and 1.0."""
        boss_data = _straight_chain()

        wave_id = "1_2_45"
        # Centroid at y=3000 -- past midfield, close to enemy guardian
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=3000.0)

        lane_creep_data = LaneCreepData(
            creeps={
                "100": _timeline_of_length(creep_snap, 1),
                "101": _timeline_of_length(creep_snap, 1),
                "102": _timeline_of_length(creep_snap, 1),
                "103": _timeline_of_length(creep_snap, 1),
            },
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        assert wave_id in result.pressure
        snap = result.pressure[wave_id][0]
        assert snap is not None
        assert 0.25 <= snap.pressure <= 1.0, (
            f"Expected pressure in [0.25, 1.0], got {snap.pressure}"
        )
        assert snap.creep_count == 4
        assert snap.team == 2


# ---------------------------------------------------------------------------
# 2)  Creeps alive at second 0, dead at second 1 -> None snapshot at second 1
# ---------------------------------------------------------------------------

class TestNoAliveCreeps:
    def test_returns_none_when_creeps_dead_at_second(self):
        """A second where all creeps in a wave are dead must produce a None snapshot."""
        boss_data = _straight_chain(timeline_length=2)

        wave_id = "1_2_45"
        creep = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=0.0)
        lane_creep_data = LaneCreepData(
            creeps={
                "100": [creep, None],
                "101": [creep, None],
            },
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        assert wave_id in result.pressure
        assert result.pressure[wave_id][0] is not None  # alive at second 0
        assert result.pressure[wave_id][1] is None       # dead at second 1


# ---------------------------------------------------------------------------
# 3)  Two of four alive -> pressure = raw * 0.5
# ---------------------------------------------------------------------------

class TestPartialAliveCreeps:
    def test_two_alive_uses_half_multiplier(self):
        """With 2 alive creeps the multiplier is 2 * 0.25 = 0.5."""
        boss_data = _straight_chain()

        wave_id = "1_2_45"
        # Centroid at midfield (y=0) -- exactly halfway along the contested zone
        creep_at = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=0.0)

        lane_creep_data = LaneCreepData(
            creeps={
                "100": [creep_at],   # alive
                "101": [creep_at],   # alive
                "102": [None],       # dead
                "103": [None],       # dead
            },
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        assert snap.creep_count == 2

        # Centroid y=0 projects onto own_guardian->enemy_guardian at t=0.5
        # progress = 10000; own_prog = 5000 (own_guardian); target_prog = 15000 (enemy_guardian)
        # raw = (10000 - 5000) / (15000 - 5000) = 0.5
        # pressure = 0.5 * (2 * 0.25) = 0.25
        expected_raw = _expected_straight_raw(
            0.0, STRAIGHT_PROG[OWN_GUARDIAN_IDX], STRAIGHT_PROG[ENEMY_GUARDIAN_IDX]
        )
        expected_pressure = expected_raw * (2 * 0.25)
        assert abs(snap.pressure - expected_pressure) < 1e-6, (
            f"Expected pressure {expected_pressure}, got {snap.pressure}"
        )


# ---------------------------------------------------------------------------
# 4)  Objective chaining -- target advances as defenders die
# ---------------------------------------------------------------------------

class TestObjectiveChaining:
    def test_target_advances_to_walker_when_guardian_dead(self):
        """Guardian dead, walker alive -> target is walker (not the dead guardian)."""
        boss_data = _straight_chain(dead_entities={ENEMY_GUARDIAN_IDX})

        wave_id = "1_2_45"
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=5500.0)

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        # target = enemy_walker at y=6000, progress = 16000
        # own_frontline = own_guardian at y=-5000, progress = 5000
        # centroid y=5500 -> progress = 15500 (segment enemy_guardian->enemy_walker at t=0.5)
        # raw = (15500 - 5000) / (16000 - 5000) = 10500 / 11000 ≈ 0.9545
        expected_raw = _expected_straight_raw(
            5500.0, STRAIGHT_PROG[OWN_GUARDIAN_IDX], STRAIGHT_PROG[ENEMY_WALKER_IDX]
        )
        expected_pressure = expected_raw * 0.25  # 1 creep
        assert abs(snap.pressure - expected_pressure) < 1e-4

    def test_target_advances_to_base_guardian_when_walker_dead(self):
        """Guardian + Walker dead -> target advances to Base Guardian."""
        boss_data = _straight_chain(
            dead_entities={ENEMY_GUARDIAN_IDX, ENEMY_WALKER_IDX}
        )

        wave_id = "1_2_45"
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=7000.0)

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        expected_raw = _expected_straight_raw(
            7000.0, STRAIGHT_PROG[OWN_GUARDIAN_IDX], STRAIGHT_PROG[ENEMY_BG_A_IDX]
        )
        expected_pressure = expected_raw * 0.25
        assert abs(snap.pressure - expected_pressure) < 1e-4

    def test_target_advances_to_shrine_when_base_guardian_dead(self):
        """Guardian + Walker + BG dead -> target advances to Shrine.

        Regression against `_build_objective_map` -- if shrines are not appended
        to per-lane buckets, this fails with target = BG (still sorted last alive).
        """
        boss_data = _straight_chain(
            dead_entities={
                ENEMY_GUARDIAN_IDX,
                ENEMY_WALKER_IDX,
                ENEMY_BG_A_IDX,
                ENEMY_BG_B_IDX,
            }
        )

        wave_id = "1_2_45"
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=8500.0)

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        # target = enemy_shrine (prog 19000)
        # own_frontline = own_guardian (prog 5000)
        # centroid y=8500 -> prog 18500 (segment enemy_bg->enemy_shrine at t=0.5)
        expected_raw = _expected_straight_raw(
            8500.0, STRAIGHT_PROG[OWN_GUARDIAN_IDX], STRAIGHT_PROG[ENEMY_SHRINE_IDX]
        )
        expected_pressure = expected_raw * 0.25
        assert abs(snap.pressure - expected_pressure) < 1e-4

    def test_target_advances_to_patron_when_shrine_dead(self):
        """Every enemy objective dead except Patron -> target is Patron."""
        boss_data = _straight_chain(
            dead_entities={
                ENEMY_GUARDIAN_IDX,
                ENEMY_WALKER_IDX,
                ENEMY_BG_A_IDX,
                ENEMY_BG_B_IDX,
                ENEMY_SHRINE_IDX,
            }
        )

        wave_id = "1_2_45"
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=9500.0)

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        expected_raw = _expected_straight_raw(
            9500.0, STRAIGHT_PROG[OWN_GUARDIAN_IDX], STRAIGHT_PROG[ENEMY_PATRON_IDX]
        )
        expected_pressure = expected_raw * 0.25
        assert abs(snap.pressure - expected_pressure) < 1e-4

    def test_lbend_lane_produces_high_pressure_past_the_bend(self):
        """Regression: match 68182475 lane 1 at ~22:40.

        Amber (team 2) creeps clustered past the enemy guardian in the bend near
        sapphire's base guardian. Pre-fix, the calc combined a stale-guardian target
        with straight-line lane_length, producing 0.0 pressure. Post-fix (boss health
        fresh + path-aware projection), pressure should be high (>=0.9) because the
        wave is right next to its real target.

        Boss coordinates are the actual values from match 68182475, so this
        exercises the L-bend path through real-world waypoints.
        """
        # Team 2 (amber) lane 1 objectives
        own_patron = _make_boss(295, BOSS_HASH_PATRON, lane=0, team=2, x=0.0, y=-8034.0)
        own_shrine_a = _make_boss(298, BOSS_HASH_SHRINE, lane=0, team=2, x=1579.0, y=-7535.0)
        own_shrine_b = _make_boss(428, BOSS_HASH_SHRINE, lane=0, team=2, x=-1579.0, y=-7535.0)
        own_bg_a = _make_boss(344, BOSS_HASH_BASE_GUARDIAN, lane=1, team=2, x=-1760.0, y=-6396.0)
        own_bg_b = _make_boss(345, BOSS_HASH_BASE_GUARDIAN, lane=1, team=2, x=-1760.0, y=-6756.0)
        own_walker = _make_boss(302, BOSS_HASH_WALKER, lane=1, team=2, x=-6272.0, y=-4736.0)
        own_guardian = _make_boss(2527, BOSS_HASH_GUARDIAN, lane=1, team=2, x=-8128.0, y=-1856.0)
        # Team 3 (sapphire) lane 1 objectives
        enemy_guardian = _make_boss(2530, BOSS_HASH_GUARDIAN, lane=1, team=3, x=-7040.0, y=1984.0)
        enemy_walker = _make_boss(299, BOSS_HASH_WALKER, lane=1, team=3, x=-5440.0, y=5024.0)
        enemy_bg_a = _make_boss(348, BOSS_HASH_BASE_GUARDIAN, lane=1, team=3, x=-1760.0, y=6396.0)
        enemy_bg_b = _make_boss(349, BOSS_HASH_BASE_GUARDIAN, lane=1, team=3, x=-1760.0, y=6788.0)
        enemy_shrine_a = _make_boss(416, BOSS_HASH_SHRINE, lane=0, team=3, x=-1536.0, y=7536.0)
        enemy_shrine_b = _make_boss(415, BOSS_HASH_SHRINE, lane=0, team=3, x=1536.0, y=7536.0)
        enemy_patron = _make_boss(294, BOSS_HASH_PATRON, lane=0, team=3, x=0.0, y=8048.0)

        boss_data = BossData(
            snapshots=[
                own_patron, own_shrine_a, own_shrine_b, own_bg_a, own_bg_b,
                own_walker, own_guardian,
                enemy_guardian, enemy_walker, enemy_bg_a, enemy_bg_b,
                enemy_shrine_a, enemy_shrine_b, enemy_patron,
            ],
            # At the scrutinized second: enemy guardian and walker dead, BGs alive,
            # shrines and patron alive. Own walker alive, own guardian dead.
            health_timeline=[{
                str(own_guardian.entity_index): 0,
                str(own_walker.entity_index): 5000,
                str(own_bg_a.entity_index): 4000,
                str(own_bg_b.entity_index): 4000,
                str(own_shrine_a.entity_index): 5000,
                str(own_shrine_b.entity_index): 5000,
                str(own_patron.entity_index): 12000,
                str(enemy_guardian.entity_index): 0,
                str(enemy_walker.entity_index): 0,
                str(enemy_bg_a.entity_index): 4000,
                str(enemy_bg_b.entity_index): 4000,
                str(enemy_shrine_a.entity_index): 5000,
                str(enemy_shrine_b.entity_index): 5000,
                str(enemy_patron.entity_index): 12000,
            }],
        )

        wave_id = "1_2_1360"
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=-2000.0, y=6600.0)

        lane_creep_data = LaneCreepData(
            creeps={f"{i}": [creep_snap] for i in range(100, 104)},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=1360)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        assert snap.pressure >= 0.9, (
            f"Expected high pressure near enemy BG, got {snap.pressure}"
        )


# ---------------------------------------------------------------------------
# 5)  Integration coverage
# ---------------------------------------------------------------------------

class TestProcessCreepWavesIntegration:
    def test_empty_lane_creep_data_returns_empty_result(self):
        """Empty creep data should return empty LanePressureData without crashing."""
        boss_data = BossData(snapshots=[], health_timeline=[])
        lane_creep_data = LaneCreepData(creeps={}, wave_meta={})

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        assert isinstance(result, LanePressureData)
        assert result.pressure == {}

    def test_two_waves_separate_keys(self):
        """Two waves in different lanes should produce two separate pressure keys."""
        # Build a fixture with both lane 1 and lane 2 populated. The lane 2
        # objectives are placed on the x-axis so they do not interfere with
        # lane 1 projection.
        fixture = _straight_chain().snapshots.copy()
        # Lane 2 gets its own full chain on the x-axis.
        lane2 = [
            _make_boss(21, BOSS_HASH_PATRON,        lane=0, team=2, x=-10000, y=0.0),
            _make_boss(22, BOSS_HASH_SHRINE,        lane=0, team=2, x=-9000,  y=0.0),
            _make_boss(23, BOSS_HASH_BASE_GUARDIAN, lane=2, team=2, x=-8000,  y=0.0),
            _make_boss(24, BOSS_HASH_WALKER,        lane=2, team=2, x=-6000,  y=0.0),
            _make_boss(25, BOSS_HASH_GUARDIAN,      lane=2, team=2, x=-5000,  y=0.0),
            _make_boss(26, BOSS_HASH_GUARDIAN,      lane=2, team=3, x=5000,   y=0.0),
            _make_boss(27, BOSS_HASH_WALKER,        lane=2, team=3, x=6000,   y=0.0),
            _make_boss(28, BOSS_HASH_BASE_GUARDIAN, lane=2, team=3, x=8000,   y=0.0),
            _make_boss(29, BOSS_HASH_SHRINE,        lane=0, team=3, x=9000,   y=0.0),
            _make_boss(30, BOSS_HASH_PATRON,        lane=0, team=3, x=10000,  y=0.0),
        ]
        # Because lane=0 entities are shared across lanes (shrine, patron), we
        # use the same team 2 and team 3 lane=0 entities from _straight_chain.
        # The x-axis patron/shrine above override position but that is fine --
        # the last-in-spec wins at snapshot-dict lookup.
        all_snaps = fixture + [b for b in lane2 if b.lane != 0 or b.entity_index > 20]
        # Build a health_timeline that marks everything alive.
        window = {str(b.entity_index): 10000 for b in all_snaps}
        boss_data = BossData(snapshots=all_snaps, health_timeline=[window])

        wave_l1 = "1_2_45"
        wave_l2 = "2_2_45"
        # Centroids past midfield for each lane.
        creep_l1 = _make_creep(lane=1, team=2, wave_id=wave_l1, x=0.0, y=1000.0)
        creep_l2 = _make_creep(lane=2, team=2, wave_id=wave_l2, x=1000.0, y=0.0)

        lane_creep_data = LaneCreepData(
            creeps={
                "100": [creep_l1],
                "200": [creep_l2],
            },
            wave_meta={
                wave_l1: WaveMeta(lane=1, team=2, spawn_sec=45),
                wave_l2: WaveMeta(lane=2, team=2, spawn_sec=45),
            },
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        assert wave_l1 in result.pressure
        assert wave_l2 in result.pressure
        assert result.pressure[wave_l1][0] is not None
        assert result.pressure[wave_l2][0] is not None

    def test_attributed_players_union_across_creeps(self):
        """attributed_players should be the union of all alive creeps' nearby_players."""
        boss_data = _straight_chain()

        wave_id = "1_2_45"
        creep_a = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=1000.0, nearby_players=[0, 1])
        creep_b = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=1100.0, nearby_players=[1, 2])

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_a], "101": [creep_b]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        assert set(snap.attributed_players) == {0, 1, 2}

    def test_pressure_clamped_to_zero_behind_own_base(self):
        """A creep that spawns behind its own patron gets 0.0 pressure.

        The own_frontline projects to own_guardian (prog 5000) and the centroid
        projects to own_patron (prog 0). (centroid_prog - own_prog) is negative,
        which clamps to 0.
        """
        boss_data = _straight_chain()

        wave_id = "1_2_0"
        # Creep at y=-11000, behind own patron (y=-10000)
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=-11000.0)

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=0)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None
        assert snap.pressure == 0.0, (
            f"Expected 0.0 pressure when creep is behind own base, got {snap.pressure}"
        )


# ---------------------------------------------------------------------------
# 6)  Degraded path handling -- loud logging and None snapshot
# ---------------------------------------------------------------------------

class TestRawPressureDegradedBranches:
    """Direct coverage for `_raw_pressure` degraded-input handling."""

    def test_fewer_than_two_waypoints_returns_none_and_logs(self, caplog):
        own = _make_boss(1, BOSS_HASH_GUARDIAN, lane=1, team=2, x=0.0, y=-5000.0)
        target = _make_boss(2, BOSS_HASH_GUARDIAN, lane=1, team=3, x=0.0, y=5000.0)
        with caplog.at_level(logging.ERROR, logger="app.services.lane_pressure_service"):
            result = LanePressureCalculator._raw_pressure(
                centroid_x=0.0,
                centroid_y=0.0,
                own_frontline=own,
                target=target,
                lane_path=[(0.0, -5000.0)],  # only one waypoint -> degraded
                wave_id="test_wave",
                second=0,
            )
        assert result is None
        assert any("Degraded lane path" in r.message for r in caplog.records)

    def test_zone_length_zero_returns_none_and_logs(self, caplog):
        """own_frontline and target projecting to the same path position yields
        a zero-width contested zone."""
        # Place own_frontline and target at the same position. They both
        # project to the same segment at the same t, so target_prog == own_prog.
        own = _make_boss(1, BOSS_HASH_GUARDIAN, lane=1, team=2, x=0.0, y=0.0)
        target = _make_boss(2, BOSS_HASH_GUARDIAN, lane=1, team=3, x=0.0, y=0.0)
        lane_path = [(0.0, -5000.0), (0.0, 5000.0)]

        with caplog.at_level(logging.ERROR, logger="app.services.lane_pressure_service"):
            result = LanePressureCalculator._raw_pressure(
                centroid_x=0.0,
                centroid_y=0.0,
                own_frontline=own,
                target=target,
                lane_path=lane_path,
                wave_id="test_wave",
                second=0,
            )
        assert result is None
        assert any("Degenerate contested zone" in r.message for r in caplog.records)


class TestDegradedLanePath:
    def test_wave_on_unknown_lane_logs_error_and_emits_none_snapshots(self, caplog):
        """A wave pointing at a lane with no objectives must not crash.

        The service logs an ERROR (with the wave id, lane, and counts of
        missing data) and emits None for every second of that wave so the
        gap is visible downstream instead of being silently zeroed.
        """
        # Full chain on lane 1, but the wave claims lane=99 -- no objectives
        # exist for that (lane, team) in the objective map or lane paths.
        boss_data = _straight_chain()

        wave_id = "99_2_0"
        creep_snap = _make_creep(lane=99, team=2, wave_id=wave_id, x=0.0, y=0.0)
        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=99, team=2, spawn_sec=0)},
        )

        with caplog.at_level(logging.ERROR, logger="app.services.lane_pressure_service"):
            result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        assert wave_id in result.pressure
        assert all(s is None for s in result.pressure[wave_id])
        assert any(
            "99_2_0" in rec.message and "lane=99" in rec.message
            for rec in caplog.records
        ), f"Expected error log referencing wave 99_2_0; got {[r.message for r in caplog.records]}"


def test_build_objective_map_sorts_by_priority():
    """Direct sort-order regression for `_build_objective_map`.

    Pins that the per-lane bucket comes back in attack-priority order:
    Guardian -> Walker -> Base Guardian -> Shrine -> Patron. If
    `_BOSS_PRIORITY` ever stops keying on the str-typed `boss_name_hash`,
    every lookup falls back to 99 and the bucket sorts arbitrarily -- a
    silent regression that does not raise. Existing
    `test_target_advances_to_*` tests catch this indirectly through the
    pressure pipeline; this test catches it at the source.
    """
    boss_data = _straight_chain()
    objective_map = LanePressureCalculator._build_objective_map(boss_data)

    enemy_lane_1 = objective_map[(1, 3)]
    hashes = [s.boss_name_hash for s in enemy_lane_1]

    assert hashes[0] == BOSS_HASH_GUARDIAN
    assert hashes[1] == BOSS_HASH_WALKER
    # Two Base Guardians collapse next, then shrine and patron at the tail.
    assert hashes[2] == BOSS_HASH_BASE_GUARDIAN
    assert hashes[3] == BOSS_HASH_BASE_GUARDIAN
    assert hashes[-2] == BOSS_HASH_SHRINE
    assert hashes[-1] == BOSS_HASH_PATRON
