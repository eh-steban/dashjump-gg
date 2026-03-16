import base64
import gzip
import json
import sys

from app.domain.boss import BossData
from app.domain.creep import LaneCreepData
from app.domain.exceptions import DeadlockAPIError, ParserServiceError
from app.domain.match_analysis import (
    ParsedAttackerVictimMap,
    ParsedMatchResponse,
    Positions,
    TransformedMatchData,
)
from app.domain.player import PlayerData
from app.repo.parsed_matches_repo import ParsedMatchesRepo
from app.services.deadlock_api_service import DeadlockAPIService
from app.services.match_data_service import MatchDataService
from app.services.parser_service import ParserService
from app.utils.http_cache import compute_etag
from app.utils.logger import get_logger

logger = get_logger(__name__)


class AnalyzeMatchUseCase:
    """
    Use case for analyzing a match.

    Orchestrates:
    - Cache checking
    - Parser service interaction (local demo check + parsing)
    - Deadlock API fallback
    - Data transformation
    - Cache storage
    """

    def __init__(
        self,
        parser_service: ParserService,
        deadlock_api_service: DeadlockAPIService,
        repo: ParsedMatchesRepo,
    ):
        self.parser_service = parser_service
        self.deadlock_api_service = deadlock_api_service
        self.repo = repo

    async def execute(
        self,
        match_id: int,
        schema_version: int,
        session,
    ) -> tuple[TransformedMatchData, str]:
        """
        Execute the match analysis use case.

        Args:
            match_id: The match ID to analyze
            schema_version: Current schema version for caching
            session: Database session

        Returns:
            (TransformedMatchData, etag) tuple

        Raises:
            ParserServiceError: If parser service fails
            DeadlockAPIError: If Deadlock API fails
        """
        # 1. Check cache
        match_data = await self.repo.get_match_data(match_id, schema_version, session)

        if match_data:
            logger.info("Cache hit for match_id=%s", match_id)
            etag = compute_etag(match_data.model_dump(), schema_version)
            return match_data, etag

        # 2. Cache miss - need to parse
        logger.info("Cache miss for match_id=%s, fetching data", match_id)

        parsed_json_resp = await self._fetch_and_parse(match_id)

        # 3. Transform parser response to domain model
        match_data, etag = await self._transform_and_store(
            match_id, schema_version, parsed_json_resp, session
        )

        return match_data, etag

    async def _fetch_and_parse(self, match_id: int) -> dict:
        """
        Attempt to parse demo from local file, fallback to Deadlock API.

        Returns:
            Parsed JSON response from parser
        """
        # Try local demo first
        has_demo = False
        local_filename = None

        try:
            has_demo, local_filename = await self.parser_service.check_demo_available(
                match_id
            )
        except ParserServiceError as e:
            logger.warning(
                "Parser check failed for match_id=%s, falling back to Deadlock API: %s",
                match_id,
                e,
            )

        if has_demo and local_filename:
            # Parse from local file
            logger.info("Match %s: Using local demo file: %s", match_id, local_filename)
            try:
                encoded_filename = base64.urlsafe_b64encode(
                    f"/parser/src/replays/{local_filename}".encode()
                ).decode()

                parsed_json_resp = await self.parser_service.parse_demo(
                    encoded_filename
                )
                logger.info("Match %s: Successfully parsed from local demo", match_id)
                return parsed_json_resp

            except ParserServiceError as e:
                logger.error(
                    "Local demo parse failed for match_id=%s, falling back to Deadlock API: %s",
                    match_id,
                    e,
                )
                # Fall through to Deadlock API

        # Fallback to Deadlock API
        logger.info("Match %s: Fetching from Deadlock API", match_id)
        demo = await self.deadlock_api_service.get_demo_url(match_id)
        replay_url = demo.get("demo_url")

        if not replay_url:
            logger.error("Replay URL not found for match_id=%s", match_id)
            raise DeadlockAPIError(f"Replay URL not found for match {match_id}")

        logger.info("Replay url (%s) for match ID: %s", replay_url, match_id)
        encoded_replay_url = base64.urlsafe_b64encode(replay_url.encode()).decode()

        parsed_json_resp = await self.parser_service.parse_demo(encoded_replay_url)
        return parsed_json_resp

    async def _transform_and_store(
        self,
        match_id: int,
        schema_version: int,
        parsed_json_resp: dict,
        session,
    ) -> tuple[TransformedMatchData, str]:
        """
        Transform parser response and store in cache.

        Returns:
            (TransformedMatchData, etag) tuple
        """
        # Log raw response size
        raw_response_size = len(json.dumps(parsed_json_resp).encode("utf-8"))
        logger.info(
            f"Match {match_id} - Raw parser response size: {raw_response_size:,} bytes "
            f"({raw_response_size / 1024:.2f} KB, {raw_response_size / (1024 * 1024):.2f} MB)"
        )

        # Parse into domain models
        players_list = [PlayerData(**p) for p in parsed_json_resp.get("players", [])]
        parsed_damage = [
            ParsedAttackerVictimMap(**d) for d in parsed_json_resp.get("damage", {})
        ]

        # Parse lane_creep_data if present (provide empty default for old cached data)
        lane_creep_raw = parsed_json_resp.get("lane_creep_data")
        lane_creep_data = (
            LaneCreepData(**lane_creep_raw)
            if lane_creep_raw
            else LaneCreepData(creeps={}, wave_meta={})
        )

        parsed_match = ParsedMatchResponse(
            total_match_time_s=parsed_json_resp.get("total_match_time_s", 0),
            match_start_time_s=parsed_json_resp.get("match_start_time_s", 0),
            damage=parsed_damage,
            players_data=players_list,
            positions=Positions(parsed_json_resp.get("positions", [])),
            bosses=BossData(**parsed_json_resp.get("bosses", {})),
            lane_creep_data=lane_creep_data,
        )

        # Log compression metrics
        compressed_parsed_match = gzip.compress(
            parsed_match.model_dump_json().encode("utf-8")
        )
        compressed_size = len(compressed_parsed_match)
        logger.info(
            f"Match {match_id} - Compressed parsed_match size: {compressed_size:,} bytes "
            f"({compressed_size / 1024:.2f} KB, {compressed_size / (1024 * 1024):.2f} MB)"
        )

        # Transform to final domain model
        match_data = MatchDataService.transform(parsed_match)

        # Log transformation metrics
        match_data_json = json.dumps(match_data.model_dump())
        match_data_size = len(match_data_json.encode("utf-8"))
        match_data_memory = sys.getsizeof(match_data)
        logger.info(
            f"Match {match_id} - match_data JSON size: {match_data_size:,} bytes "
            f"({match_data_size / 1024:.2f} KB, {match_data_size / (1024 * 1024):.2f} MB)"
        )
        logger.info(
            f"Match {match_id} - match_data in-memory size: {match_data_memory:,} bytes "
            f"({match_data_memory / 1024:.2f} KB, {match_data_memory / (1024 * 1024):.2f} MB)"
        )

        # Log compression ratio
        uncompressed_size = len(parsed_match.model_dump_json().encode("utf-8"))
        compression_ratio = (1 - compressed_size / uncompressed_size) * 100
        logger.info(
            f"Match {match_id} - Compression ratio: {compression_ratio:.1f}% "
            f"(uncompressed: {uncompressed_size:,} bytes, {uncompressed_size / 1024:.2f} KB, "
            f"{uncompressed_size / (1024 * 1024):.2f} MB)"
        )

        # Store in cache
        etag = compute_etag(match_data.model_dump(), schema_version)
        await self.repo.create_parsed_match(
            match_id,
            schema_version,
            gzip.compress(parsed_match.model_dump_json().encode("utf-8")),
            match_data.model_dump(),
            etag,
            session,
        )

        return match_data, etag
