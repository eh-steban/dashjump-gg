import pytest
from unittest.mock import AsyncMock, MagicMock
from app.application.use_cases.analyze_match import AnalyzeMatchUseCase
from app.domain.exceptions import ParserServiceError, DeadlockAPIError
from app.domain.match_analysis import TransformedMatchData
from app.domain.boss import BossData
from app.domain.creep import LaneCreepData
from app.domain.mid_boss import MidBossData
from app.domain.sinner import SinnerSnapshot


_MID_BOSS_DICT = {"boss_name_hash": "11298616958347856125"}


@pytest.mark.asyncio
async def test_execute_returns_cached_data_when_available():
    """Test that cached data is returned immediately."""
    mock_parser = AsyncMock()
    mock_deadlock = AsyncMock()
    mock_repo = AsyncMock()

    cached_data = TransformedMatchData(
        match_duration_s=0,
        match_start_time_s=0,
        players_data=[],
        per_player_data={},
        bosses=BossData(
            snapshots=[],
            health_timeline=[],
        ),
        lane_creep_data=LaneCreepData(creeps={}, wave_meta={}),
        sinners=[],
        mid_boss=MidBossData(**_MID_BOSS_DICT),
    )
    mock_repo.get_match_data.return_value = cached_data

    use_case = AnalyzeMatchUseCase(mock_parser, mock_deadlock, mock_repo)
    result, etag = await use_case.execute(12345, schema_version=1, session=MagicMock())

    assert result == cached_data
    assert etag is not None
    # Should not call parser or API
    mock_parser.check_demo_available.assert_not_called()
    mock_deadlock.get_demo_url.assert_not_called()


@pytest.mark.asyncio
async def test_execute_uses_local_demo_when_available():
    """Test that local demo is used when parser has it."""
    mock_parser = AsyncMock()
    mock_deadlock = AsyncMock()
    mock_repo = AsyncMock()

    mock_repo.get_match_data.return_value = None  # Cache miss
    mock_parser.check_demo_available.return_value = (True, "12345_67890.dem")
    mock_parser.parse_demo.return_value = {
        "match_duration_s": 0,
        "match_start_time_s": 0,
        "players": [],
        "damage": [],
        "positions": [],
        "bosses": {"snapshots": [], "health_timeline": []},
        "lane_creep_data": {"creeps": {}, "wave_meta": {}},
        "mid_boss": _MID_BOSS_DICT,
    }

    use_case = AnalyzeMatchUseCase(mock_parser, mock_deadlock, mock_repo)
    result, etag = await use_case.execute(12345, schema_version=1, session=MagicMock())

    # Should use local demo
    mock_parser.check_demo_available.assert_called_once_with(12345)
    mock_parser.parse_demo.assert_called_once()
    # Should NOT call Deadlock API
    mock_deadlock.get_demo_url.assert_not_called()


@pytest.mark.asyncio
async def test_execute_falls_back_to_api_when_parser_check_fails():
    """Test fallback to Deadlock API when parser check fails."""
    mock_parser = AsyncMock()
    mock_deadlock = AsyncMock()
    mock_repo = AsyncMock()

    mock_repo.get_match_data.return_value = None
    mock_parser.check_demo_available.side_effect = ParserServiceError("timeout")
    mock_deadlock.get_demo_url.return_value = {"demo_url": "http://example.com/demo.bz2"}
    mock_parser.parse_demo.return_value = {
        "match_duration_s": 0,
        "match_start_time_s": 0,
        "players": [],
        "damage": [],
        "positions": [],
        "bosses": {"snapshots": [], "health_timeline": []},
        "lane_creep_data": {"creeps": {}, "wave_meta": {}},
        "mid_boss": _MID_BOSS_DICT,
    }

    use_case = AnalyzeMatchUseCase(mock_parser, mock_deadlock, mock_repo)
    result, etag = await use_case.execute(12345, schema_version=1, session=MagicMock())

    # Should fall back to Deadlock API
    mock_deadlock.get_demo_url.assert_called_once_with(12345)


@pytest.mark.asyncio
async def test_execute_falls_back_when_local_parse_fails():
    """Test fallback when local demo exists but parsing fails."""
    mock_parser = AsyncMock()
    mock_deadlock = AsyncMock()
    mock_repo = AsyncMock()

    mock_repo.get_match_data.return_value = None
    mock_parser.check_demo_available.return_value = (True, "12345_67890.dem")
    mock_parser.parse_demo.side_effect = [
        ParserServiceError("parse failed"),  # Local parse fails
        {  # Remote parse succeeds
            "match_duration_s": 0,
            "match_start_time_s": 0,
            "players": [],
            "damage": [],
            "positions": [],
            "bosses": {"snapshots": [], "health_timeline": []},
            "lane_creep_data": {"creeps": {}, "wave_meta": {}},
            "sinners": [],
            "mid_boss": _MID_BOSS_DICT,
        },
    ]
    mock_deadlock.get_demo_url.return_value = {"demo_url": "http://example.com/demo.bz2"}

    use_case = AnalyzeMatchUseCase(mock_parser, mock_deadlock, mock_repo)
    result, etag = await use_case.execute(12345, schema_version=1, session=MagicMock())

    # Should try local first, then fall back
    assert mock_parser.parse_demo.call_count == 2
    mock_deadlock.get_demo_url.assert_called_once_with(12345)


@pytest.mark.asyncio
async def test_execute_raises_when_all_sources_fail():
    """Test that exception is raised when all sources fail."""
    mock_parser = AsyncMock()
    mock_deadlock = AsyncMock()
    mock_repo = AsyncMock()

    mock_repo.get_match_data.return_value = None
    mock_parser.check_demo_available.return_value = (False, None)
    mock_deadlock.get_demo_url.return_value = {"demo_url": "http://example.com/demo.bz2"}
    mock_parser.parse_demo.side_effect = ParserServiceError("parse failed")

    use_case = AnalyzeMatchUseCase(mock_parser, mock_deadlock, mock_repo)

    with pytest.raises(ParserServiceError):
        await use_case.execute(12345, schema_version=1, session=MagicMock())
