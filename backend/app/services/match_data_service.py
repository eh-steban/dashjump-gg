from app.domain.match_analysis import ParsedMatchResponse, TransformedMatchData
from app.services.lane_pressure_service import LanePressureCalculator
from app.services.players_data_service import PlayersDataService
from app.utils.logger import get_logger

logger = get_logger(__name__)


class MatchDataService:
    """Transform parsed match data into final domain model."""

    @staticmethod
    def transform(parsed_match: ParsedMatchResponse) -> TransformedMatchData:
        """
        Transform ParsedMatchResponse into TransformedMatchData.

        Assembles complete match data including:
        - Per-player position data
        - Per-player damage data
        - Per-creep lane tracking data (per entity)
        - Lane pressure metrics (derived from per-creep positions and boss objectives)

        Args:
            parsed_match: Parser output with raw position, damage, and creep data

        Returns:
            Complete TransformedMatchData ready for storage and API response
        """
        # Derive lane pressure from per-creep data and boss objectives
        lane_pressure = LanePressureCalculator.process_creep_waves(
            parsed_match.lane_creep_data,
            parsed_match.bosses,
        )

        # Aggregate per-player position and damage data
        per_player_data = PlayersDataService.aggregate(parsed_match)

        # Assemble final structure
        return TransformedMatchData(
            match_duration_s=parsed_match.match_duration_s,
            match_start_time_s=parsed_match.match_start_time_s,
            players_data=parsed_match.players_data,
            per_player_data=per_player_data,
            bosses=parsed_match.bosses,
            lane_creep_data=parsed_match.lane_creep_data,
            sinners=parsed_match.sinners,
            mid_boss=parsed_match.mid_boss,
            lane_pressure=lane_pressure,
        )
