"""Unit and integration tests for LanePressureCalculator.

Test spec:
- Unit: wave with 4 alive creeps near enemy guardian → pressure between 0.25 and 1.0
- Unit: wave with creeps alive at second 0 but dead at second 1 → None snapshot at second 1
- Unit: wave with 2 of 4 alive → pressure_value = raw × 0.5 (half multiplier)
- Unit: guardian destroyed → pressure calculated against walker position instead
- Integration: full process_creep_waves() call with mock data
"""

import math
import pytest

from app.domain.boss import BossData, BossSnapshot
from app.domain.creep import CreepSnapshot, LaneCreepData, WaveMeta
from app.domain.lane_pressure import LanePressureData, LanePressureSnapshot
from app.services.lane_pressure_service import LanePressureCalculator, _euclidean


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

def _make_boss(
    entity_index: int,
    custom_id: int,
    lane: int,
    team: int,
    x: float,
    y: float,
) -> BossSnapshot:
    return BossSnapshot(
        entity_index=entity_index,
        custom_id=custom_id,
        boss_name_hash=custom_id,
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
    nearby_players: list[int] | None = None,
) -> CreepSnapshot:
    return CreepSnapshot(
        x=x,
        y=y,
        lane=lane,
        team=team,
        wave_id=wave_id,
        nearby_players=nearby_players or [],
    )


def _timeline_of_length(
    snap: CreepSnapshot | None,
    length: int,
) -> list[CreepSnapshot | None]:
    """Create a timeline where every second has the same snapshot."""
    return [snap] * length


# ---------------------------------------------------------------------------
# 1)  Four alive creeps near enemy guardian → pressure between 0.25 and 1.0
# ---------------------------------------------------------------------------

class TestPressureWithAliveCreeps:
    def test_pressure_in_valid_range(self):
        """Four alive creeps close to enemy guardian should produce pressure between 0.25 and 1.0."""
        # Lane 1, team 2 creeps push toward team 3's guardian
        # Guardian team 2 at y=-5000, Guardian team 3 at y=5000 → lane_length ≈ 10000
        own_guardian = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-5000.0)
        enemy_guardian = _make_boss(2, 21, lane=1, team=3, x=0.0, y=5000.0)
        boss_data = BossData(
            snapshots=[own_guardian, enemy_guardian],
            health_timeline=[{"2": 10000}],  # enemy guardian alive at second 0
        )

        wave_id = "1_2_45"
        # Creep centroid at y=3000 — relatively close to enemy guardian at y=5000
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
        assert 0.25 <= snap.pressure <= 1.0, f"Expected pressure in [0.25, 1.0], got {snap.pressure}"
        assert snap.creep_count == 4
        assert snap.team == 2


# ---------------------------------------------------------------------------
# 2)  Creeps alive at second 0, dead at second 1 → None snapshot at second 1
# ---------------------------------------------------------------------------

class TestNoAliveCreeps:
    def test_returns_none_when_creeps_dead_at_second(self):
        """A second where all creeps in a wave are dead must produce a None snapshot."""
        own_guardian = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-5000.0)
        enemy_guardian = _make_boss(2, 21, lane=1, team=3, x=0.0, y=5000.0)
        boss_data = BossData(
            snapshots=[own_guardian, enemy_guardian],
            health_timeline=[{"2": 10000}, {"2": 10000}],
        )

        wave_id = "1_2_45"
        creep = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=0.0)
        # Creeps alive at second 0, dead (None) at second 1
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
# 3)  Two of four alive → pressure = raw × 0.5
# ---------------------------------------------------------------------------

class TestPartialAliveCreeps:
    def test_two_alive_uses_half_multiplier(self):
        """With 2 alive creeps the multiplier is 2 × 0.25 = 0.5."""
        own_guardian = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-5000.0)
        enemy_guardian = _make_boss(2, 21, lane=1, team=3, x=0.0, y=5000.0)
        boss_data = BossData(
            snapshots=[own_guardian, enemy_guardian],
            health_timeline=[{"2": 10000}],
        )

        wave_id = "1_2_45"
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

        # Compute expected raw_pressure manually
        lane_length = _euclidean(0.0, -5000.0, 0.0, 5000.0)  # 10000
        dist = _euclidean(0.0, 0.0, 0.0, 5000.0)             # 5000
        expected_raw = max(0.0, min(1.0, 1.0 - dist / lane_length))   # 0.5
        expected_pressure = expected_raw * (2 * 0.25)                  # 0.5 * 0.5 = 0.25

        assert abs(snap.pressure - expected_pressure) < 1e-6, (
            f"Expected pressure {expected_pressure}, got {snap.pressure}"
        )


# ---------------------------------------------------------------------------
# 4)  Guardian destroyed → pressure calculated against walker
# ---------------------------------------------------------------------------

class TestObjectiveChaining:
    def test_pressure_targets_walker_when_guardian_dead(self):
        """When enemy guardian has 0 health, the next objective (walker) should be targeted."""
        own_guardian = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-5000.0)
        enemy_guardian = _make_boss(2, 21, lane=1, team=3, x=0.0, y=4000.0)
        enemy_walker = _make_boss(3, 25, lane=1, team=3, x=0.0, y=7000.0)
        boss_data = BossData(
            snapshots=[own_guardian, enemy_guardian, enemy_walker],
            # At second 0: enemy guardian dead (0 health), walker alive
            health_timeline=[{"2": 0, "3": 10000}],
        )

        wave_id = "1_2_45"
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=5500.0)

        lane_creep_data = LaneCreepData(
            creeps={"100": [creep_snap]},
            wave_meta={wave_id: WaveMeta(lane=1, team=2, spawn_sec=45)},
        )

        result = LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)

        snap = result.pressure[wave_id][0]
        assert snap is not None

        # The pressure was computed against the walker (y=7000), not the dead guardian (y=4000)
        # lane_length spans own frontline to the current live target (walker), not the dead guardian
        lane_length = _euclidean(0.0, -5000.0, 0.0, 7000.0)  # own guardian to enemy walker
        dist_to_walker = _euclidean(0.0, 5500.0, 0.0, 7000.0)
        expected_raw = max(0.0, min(1.0, 1.0 - dist_to_walker / lane_length))
        expected_pressure = expected_raw * 0.25  # 1 creep

        assert abs(snap.pressure - expected_pressure) < 1e-4, (
            f"Expected pressure {expected_pressure} (vs walker), got {snap.pressure}"
        )


# ---------------------------------------------------------------------------
# 5)  Integration: full process_creep_waves call
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
        own_g_l1 = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-5000.0)
        emy_g_l1 = _make_boss(2, 21, lane=1, team=3, x=0.0, y=5000.0)
        own_g_l2 = _make_boss(3, 21, lane=2, team=2, x=-5000.0, y=0.0)
        emy_g_l2 = _make_boss(4, 21, lane=2, team=3, x=5000.0, y=0.0)

        boss_data = BossData(
            snapshots=[own_g_l1, emy_g_l1, own_g_l2, emy_g_l2],
            health_timeline=[{"2": 10000, "4": 10000}],
        )

        wave_l1 = "1_2_45"
        wave_l2 = "2_2_45"

        creep_l1 = _make_creep(lane=1, team=2, wave_id=wave_l1, x=0.0, y=1000.0)
        creep_l2 = _make_creep(lane=2, team=2, wave_id=wave_l2, x=1000.0, y=0.0)

        lane_creep_data = LaneCreepData(
            creeps={
                "10": [creep_l1],
                "20": [creep_l2],
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
        own_guardian = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-5000.0)
        enemy_guardian = _make_boss(2, 21, lane=1, team=3, x=0.0, y=5000.0)
        boss_data = BossData(
            snapshots=[own_guardian, enemy_guardian],
            health_timeline=[{"2": 10000}],
        )

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
        """A creep that spawns behind its own guardian (negative territory) gets 0.0 pressure.

        Place creep at y=-10000, which is beyond the own guardian at y=-9000
        (further from enemy). dist(creep→enemy_guardian) > lane_length → clamped to 0.
        """
        own_guardian = _make_boss(1, 21, lane=1, team=2, x=0.0, y=-9000.0)
        enemy_guardian = _make_boss(2, 21, lane=1, team=3, x=0.0, y=9000.0)
        boss_data = BossData(
            snapshots=[own_guardian, enemy_guardian],
            health_timeline=[{"2": 10000}],
        )

        wave_id = "1_2_0"
        # Creep is behind own guardian — farther from enemy than the full lane length
        creep_snap = _make_creep(lane=1, team=2, wave_id=wave_id, x=0.0, y=-10000.0)

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
