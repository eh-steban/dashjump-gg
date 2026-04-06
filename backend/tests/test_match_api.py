"""Tests for the match API -- schema contracts and route error paths.

Two test categories:
1. Schema contract -- verify MatchAnalysis and nested types have the expected fields.
   These catch field renames/removals before they reach the frontend.
2. Route error paths -- verify correct HTTP status codes for each exception type.
"""
import pytest
from unittest.mock import AsyncMock, patch
from httpx import ASGITransport, AsyncClient

from app.main import app
from app.domain.match_analysis import MatchAnalysis, TransformedMatchData
from app.domain.player import PlayerData, DamageRecord
from app.domain.boss import BossData
from app.domain.creep import LaneCreepData
from app.domain.deadlock_api import MatchMetadata, MatchInfoFields, MatchPaths
from app.domain.lane_pressure import LanePressureData
from app.domain.exceptions import (
    DeadlockAPIError,
    MatchDataIntegrityException,
    MatchDataUnavailableException,
    ParserServiceError,
)
from app.infra.db.session import get_db_session
from app.application.use_cases.analyze_match import AnalyzeMatchUseCase


# ---------------------------------------------------------------------------
# Helpers -- minimal valid domain objects
# ---------------------------------------------------------------------------

def _make_transformed_match_data() -> TransformedMatchData:
    return TransformedMatchData(
        match_duration_s=1800,
        match_start_time_s=0,
        players_data=[],
        per_player_data={},
        bosses=BossData(snapshots=[], health_timeline=[]),
        lane_creep_data=LaneCreepData(creeps={}, wave_meta={}),
    )


def _make_match_metadata() -> MatchMetadata:
    return MatchMetadata(
        match_info=MatchInfoFields(
            duration_s=1800,
            match_outcome=1,
            winning_team=2,
            players=[],
            start_time=1700000000,
            match_id=12345,
            legacy_objectives_mask=None,
            game_mode=1,
            match_mode=1,
            objectives=[],
            match_paths=MatchPaths(x_resolution=16383, y_resolution=16383),
            damage_matrix={},
            match_pauses=[],
            watched_death_replays=[],
            mid_boss=[],
            is_high_skill_range_parties=False,
            low_pri_pool=False,
            new_player_pool=False,
            average_badge_team0=90,
            average_badge_team1=90,
            game_mode_version=1,
        )
    )


def _make_match_analysis() -> MatchAnalysis:
    return MatchAnalysis(
        match_metadata=_make_match_metadata(),
        parsed_match_data=_make_transformed_match_data(),
    )


async def _override_db_session():
    yield AsyncMock()


# ---------------------------------------------------------------------------
# 1) Schema contract tests -- pure domain, no HTTP
# ---------------------------------------------------------------------------

class TestMatchAnalysisSchema:
    """Verify MatchAnalysis and nested types have the expected field shapes.

    If a field is renamed or removed in backend domain models, these tests
    catch it before the change silently breaks the frontend.
    """

    def test_match_analysis_top_level_fields(self):
        analysis = _make_match_analysis()
        assert hasattr(analysis, "match_metadata")
        assert hasattr(analysis, "parsed_match_data")

    def test_transformed_match_data_fields(self):
        data = _make_transformed_match_data()
        assert hasattr(data, "match_duration_s")
        assert hasattr(data, "match_start_time_s")
        assert hasattr(data, "players_data")
        assert hasattr(data, "per_player_data")
        assert hasattr(data, "bosses")
        assert hasattr(data, "lane_creep_data")
        assert hasattr(data, "lane_pressure")

    def test_lane_pressure_defaults_to_empty(self):
        data = _make_transformed_match_data()
        assert isinstance(data.lane_pressure, LanePressureData)
        assert data.lane_pressure.pressure == {}

    def test_player_data_required_fields(self):
        player = PlayerData(entity_id="1", custom_id="42", name="Player", team=2, lane=1)
        assert player.entity_id == "1"
        assert player.custom_id == "42"
        assert player.name == "Player"
        assert player.team == 2
        assert player.lane == 1

    def test_player_data_optional_fields_default_to_none(self):
        player = PlayerData(entity_id="1", custom_id="42", name="Player", team=2, lane=1)
        assert player.steam_id_32 is None
        assert player.hero_id is None
        assert player.lobby_player_slot is None

    def test_damage_record_all_fields_optional(self):
        record = DamageRecord()
        assert record.damage is None
        assert record.health_lost is None
        assert record.hits is None

    def test_match_analysis_serializes_to_dict(self):
        """model_dump() must succeed -- validates the full JSON serialization path."""
        d = _make_match_analysis().model_dump()
        assert "match_metadata" in d
        assert "parsed_match_data" in d
        pd = d["parsed_match_data"]
        assert "match_duration_s" in pd
        assert "players_data" in pd
        assert "per_player_data" in pd
        assert "bosses" in pd
        assert "lane_creep_data" in pd
        assert "lane_pressure" in pd


# ---------------------------------------------------------------------------
# 2) Route error path tests
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_deadlock_api_error_returns_404():
    app.dependency_overrides[get_db_session] = _override_db_session
    try:
        with patch.object(
            AnalyzeMatchUseCase, "execute",
            new_callable=AsyncMock,
            side_effect=DeadlockAPIError("not found"),
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                response = await client.get("/match/analysis/12345")
        assert response.status_code == 404
    finally:
        app.dependency_overrides.pop(get_db_session, None)


@pytest.mark.asyncio
async def test_parser_service_error_returns_502():
    app.dependency_overrides[get_db_session] = _override_db_session
    try:
        with patch.object(
            AnalyzeMatchUseCase, "execute",
            new_callable=AsyncMock,
            side_effect=ParserServiceError("parser down"),
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                response = await client.get("/match/analysis/12345")
        assert response.status_code == 502
    finally:
        app.dependency_overrides.pop(get_db_session, None)


@pytest.mark.asyncio
async def test_match_data_unavailable_returns_500():
    app.dependency_overrides[get_db_session] = _override_db_session
    try:
        with patch.object(
            AnalyzeMatchUseCase, "execute",
            new_callable=AsyncMock,
            side_effect=MatchDataUnavailableException("missing"),
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                response = await client.get("/match/analysis/12345")
        assert response.status_code == 500
    finally:
        app.dependency_overrides.pop(get_db_session, None)


@pytest.mark.asyncio
async def test_match_data_integrity_error_returns_500():
    app.dependency_overrides[get_db_session] = _override_db_session
    try:
        with patch.object(
            AnalyzeMatchUseCase, "execute",
            new_callable=AsyncMock,
            side_effect=MatchDataIntegrityException("integrity error"),
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                response = await client.get("/match/analysis/12345")
        assert response.status_code == 500
    finally:
        app.dependency_overrides.pop(get_db_session, None)


@pytest.mark.asyncio
async def test_unhandled_exception_returns_500():
    app.dependency_overrides[get_db_session] = _override_db_session
    try:
        with patch.object(
            AnalyzeMatchUseCase, "execute",
            new_callable=AsyncMock,
            side_effect=RuntimeError("unexpected failure"),
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                response = await client.get("/match/analysis/12345")
        assert response.status_code == 500
    finally:
        app.dependency_overrides.pop(get_db_session, None)
