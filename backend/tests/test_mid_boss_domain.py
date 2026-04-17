"""Domain-level round-trip tests for the MidBossData contract.

Mid-boss spawns in every Deadlock match (10-minute timer), so the real
empty-downstream path is "spawn event present, nothing else". These tests
verify the Pydantic boundary preserves all six contract keys on a spawn-only
payload so the frontend can read them unconditionally, and that `max_health`
lives on each spawn event (per-cycle scaling) rather than as a top-level field.
"""

from app.domain.mid_boss import MidBossData, MidBossSpawnEvent


MID_BOSS_CONTRACT_KEYS = {
    "boss_name_hash",
    "spawn_events",
    "kill_events",
    "rejuv_events",
    "fight_windows",
    "post_match",
}

MID_BOSS_SPAWN_EVENT_KEYS = {"spawn_cycle", "spawn_time_s", "max_health"}


def test_mid_boss_data_round_trips_spawn_without_kill():
    original = MidBossData(
        boss_name_hash="11298616958347856125",
        spawn_events=[
            MidBossSpawnEvent(spawn_cycle=1, spawn_time_s=600.0, max_health=14950)
        ],
        kill_events=[],
        rejuv_events=[],
        fight_windows=[],
        post_match=[],
    )

    payload = original.model_dump()

    assert set(payload.keys()) == MID_BOSS_CONTRACT_KEYS
    assert payload["spawn_events"] == [
        {"spawn_cycle": 1, "spawn_time_s": 600.0, "max_health": 14950}
    ]
    assert set(payload["spawn_events"][0].keys()) == MID_BOSS_SPAWN_EVENT_KEYS
    assert payload["kill_events"] == []
    assert payload["rejuv_events"] == []
    assert payload["fight_windows"] == []
    assert payload["post_match"] == []

    restored = MidBossData.model_validate(payload)
    assert restored == original


def test_mid_boss_spawn_events_carry_distinct_per_cycle_max_health():
    data = MidBossData(
        boss_name_hash="11298616958347856125",
        spawn_events=[
            MidBossSpawnEvent(spawn_cycle=1, spawn_time_s=600.0, max_health=14950),
            MidBossSpawnEvent(spawn_cycle=2, spawn_time_s=1200.0, max_health=16900),
            MidBossSpawnEvent(spawn_cycle=3, spawn_time_s=1800.0, max_health=18850),
        ],
    )

    payload = data.model_dump()

    assert [s["max_health"] for s in payload["spawn_events"]] == [14950, 16900, 18850]
    assert "max_health" not in payload
