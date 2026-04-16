"""Domain-level round-trip tests for the MidBossData contract.

Mid-boss spawns in every Deadlock match (10-minute timer), so the real
empty-downstream path is "spawn event present, nothing else". This test
verifies the Pydantic boundary preserves all seven contract keys on a
spawn-only payload so the frontend can read them unconditionally.
"""

from app.domain.mid_boss import MidBossData, MidBossSpawnEvent


MID_BOSS_CONTRACT_KEYS = {
    "boss_name_hash",
    "max_health",
    "spawn_events",
    "kill_events",
    "rejuv_events",
    "fight_windows",
    "post_match",
}


def test_mid_boss_data_round_trips_spawn_without_kill():
    original = MidBossData(
        boss_name_hash="11298616958347856125",
        max_health=13000,
        spawn_events=[MidBossSpawnEvent(spawn_cycle=1, spawn_time_s=600.0)],
        kill_events=[],
        rejuv_events=[],
        fight_windows=[],
        post_match=[],
    )

    payload = original.model_dump()

    assert set(payload.keys()) == MID_BOSS_CONTRACT_KEYS
    assert payload["spawn_events"] == [{"spawn_cycle": 1, "spawn_time_s": 600.0}]
    assert payload["kill_events"] == []
    assert payload["rejuv_events"] == []
    assert payload["fight_windows"] == []
    assert payload["post_match"] == []

    restored = MidBossData.model_validate(payload)
    assert restored == original
