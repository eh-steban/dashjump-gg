"""Tests for MatchDataService -- the pure transformation layer.

This service is a thin assembler: it fans out derived fields (lane pressure,
per-player aggregation) and passes the rest through from ParsedMatchResponse
to TransformedMatchData. The passthrough pins are what catches silent
regressions when new fields are added.
"""
from app.domain.boss import BossData
from app.domain.creep import LaneCreepData
from app.domain.match_analysis import ParsedMatchResponse, Positions
from app.domain.mid_boss import MidBossData, MidBossKillEvent, MidBossSpawnEvent
from app.services.match_data_service import MatchDataService


def _minimal_parsed_match(mid_boss: MidBossData) -> ParsedMatchResponse:
    return ParsedMatchResponse(
        match_duration_s=1800,
        match_start_time_s=0,
        damage=[],
        players_data=[],
        positions=Positions([]),
        bosses=BossData(snapshots=[], health_timeline=[]),
        lane_creep_data=LaneCreepData(creeps={}, wave_meta={}),
        sinners=[],
        mid_boss=mid_boss,
    )


def test_transform_passes_mid_boss_through_unchanged():
    """MatchDataService.transform must forward mid_boss without mutation.

    This pins the passthrough: if someone accidentally drops the assignment
    or wraps mid_boss in a transformation, this test fails loudly.
    """
    mid_boss = MidBossData(
        boss_name_hash="11298616958347856125",
        max_health=14950,
        spawn_events=[MidBossSpawnEvent(spawn_cycle=1, spawn_time_s=600.0)],
        kill_events=[
            MidBossKillEvent(
                spawn_cycle=1,
                team=2,
                team_claimed=3,
                matchtime_s=942.5,
                x=0.0,
                y=0.0,
                z=-768.0,
                bosses_remaining=0,
            )
        ],
    )
    parsed_match = _minimal_parsed_match(mid_boss)

    result = MatchDataService.transform(parsed_match)

    # Identity check -- the service should not construct a new MidBossData
    assert result.mid_boss is mid_boss


def test_transform_forwards_empty_mid_boss():
    """An empty (no-events) mid_boss should also forward identically."""
    mid_boss = MidBossData(boss_name_hash="11298616958347856125")
    parsed_match = _minimal_parsed_match(mid_boss)

    result = MatchDataService.transform(parsed_match)

    assert result.mid_boss is mid_boss
    assert result.mid_boss.spawn_events == []
    assert result.mid_boss.kill_events == []
