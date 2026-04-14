"""Tests for ParsedMatchesRepo -- storage and upsert behaviour.

Uses a real test DB (deadlock_test_db) via fixtures in test_helper.py.
"""
from unittest.mock import AsyncMock

import pytest
from sqlalchemy.exc import IntegrityError
from tests.test_helper import setup_database, async_session  # noqa: F401 (fixtures)
from app.repo.parsed_matches_repo import ParsedMatchesRepo
from app.domain.boss import BossData
from app.domain.creep import LaneCreepData
from app.domain.match_analysis import TransformedMatchData
from app.domain.lane_pressure import LanePressureData
from app.domain.mid_boss import MidBossData


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _match_data_dict(match_duration_s: int = 1800) -> dict:
    """Minimal valid TransformedMatchData serialized to dict."""
    return TransformedMatchData(
        match_duration_s=match_duration_s,
        match_start_time_s=0,
        players_data=[],
        per_player_data={},
        bosses=BossData(snapshots=[], health_timeline=[]),
        lane_creep_data=LaneCreepData(creeps={}, wave_meta={}),
        lane_pressure=LanePressureData(pressure={}),
        mid_boss=MidBossData(boss_name_hash="11298616958347856125"),
    ).model_dump()


SCHEMA_VERSION = 1


# ---------------------------------------------------------------------------
# PR-1: Insert new match -- can be read back
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
class TestCreateParsedMatch:
    async def test_insert_new_match_can_be_read_back(self, async_session):
        repo = ParsedMatchesRepo()
        match_data = _match_data_dict(match_duration_s=1800)

        await repo.create_parsed_match(
            match_id=99001,
            schema_version=SCHEMA_VERSION,
            raw_payload_gzip=b"fake-gzip",
            match_data=match_data,
            etag="etag-v1",
            session=async_session,
        )

        result = await repo.get_match_data(99001, SCHEMA_VERSION, async_session)
        assert result is not None
        assert result.match_duration_s == 1800

    # ---------------------------------------------------------------------------
    # PR-2: Upsert -- re-insert same match_id updates the stored row
    # ---------------------------------------------------------------------------

    async def test_upsert_overwrites_stored_data_on_conflict(self, async_session):
        repo = ParsedMatchesRepo()

        # First parse
        await repo.create_parsed_match(
            match_id=99002,
            schema_version=SCHEMA_VERSION,
            raw_payload_gzip=b"gzip-v1",
            match_data=_match_data_dict(match_duration_s=1800),
            etag="etag-v1",
            session=async_session,
        )

        # Re-parse with updated data (e.g. after a parser fix)
        await repo.create_parsed_match(
            match_id=99002,
            schema_version=SCHEMA_VERSION,
            raw_payload_gzip=b"gzip-v2",
            match_data=_match_data_dict(match_duration_s=2400),
            etag="etag-v2",
            session=async_session,
        )

        result = await repo.get_match_data(99002, SCHEMA_VERSION, async_session)
        assert result is not None
        assert result.match_duration_s == 2400, (
            "Re-parse should overwrite stored data; got stale value"
        )

    # ---------------------------------------------------------------------------
    # PR-3: Cache miss -- get_match_data returns None for unknown match
    # ---------------------------------------------------------------------------

    async def test_get_match_data_returns_none_for_unknown_match(self, async_session):
        repo = ParsedMatchesRepo()
        result = await repo.get_match_data(99999, SCHEMA_VERSION, async_session)
        assert result is None

    # ---------------------------------------------------------------------------
    # PR-4: Concurrent-insert race -- IntegrityError is swallowed, not surfaced
    # ---------------------------------------------------------------------------
    #
    # Regression: overlapping unique indexes on parsedmatch (PK on match_id +
    # composite uq_match_id_schema_version) make ON CONFLICT arbiter selection
    # non-deterministic when two requests race to insert the same match. The repo
    # must treat the racing IntegrityError as "another request persisted first"
    # rather than propagating a 500 to the caller. The next read still hits the
    # row that was written by the winning request.
    async def test_concurrent_race_integrity_error_is_swallowed(self):
        mock_session = AsyncMock()
        mock_session.execute.side_effect = IntegrityError(
            statement="INSERT INTO parsedmatch ...",
            params={},
            orig=Exception(
                'duplicate key value violates unique constraint "uq_match_id_schema_version"'
            ),
        )

        repo = ParsedMatchesRepo()

        # Must not raise -- the row was written by the winning concurrent request.
        await repo.create_parsed_match(
            match_id=99003,
            schema_version=SCHEMA_VERSION,
            raw_payload_gzip=b"gzip",
            match_data=_match_data_dict(),
            etag="etag",
            session=mock_session,
        )

        mock_session.rollback.assert_awaited_once()
        mock_session.commit.assert_not_awaited()
