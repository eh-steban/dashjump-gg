from app.domain.match_analysis import ParsedMatchResponse, PlayerMatchData
from app.utils.logger import get_logger

logger = get_logger(__name__)

# Custom IDs below this threshold indicate human players
HUMAN_PLAYER_ID_THRESHOLD = 20


class PlayersDataService:
    """Aggregate per-player position and damage data from parsed match."""

    @staticmethod
    def aggregate(parsed_match: ParsedMatchResponse) -> dict[str, PlayerMatchData]:
        """
        Aggregate per-player positions and damage data.

        Processes match timeline in a single pass for performance, aggregating:
        - Position data for human players (custom_id < 20)
        - Damage data for all players

        Args:
            parsed_match: Parser output with position and damage data

        Returns:
            Complete dict of per-player data, ready for TransformedMatchData
        """
        # Initialize per-player data structures
        per_player_data = {
            player.custom_id: PlayerMatchData.model_construct(positions=[], damage=[])
            for player in parsed_match.players_data
        }

        # Aggregate in single timeline pass
        # positions are game-relative (pre-game stripped by parser), so offset by match_start_time_s
        match_duration = parsed_match.total_match_time_s - parsed_match.match_start_time_s

        for tick in range(match_duration):
            # Aggregate positions for human players
            for player_position in parsed_match.positions[tick]:
                custom_id = player_position.custom_id

                # Only track human players (NPCs have IDs >= 20)
                if int(custom_id) < HUMAN_PLAYER_ID_THRESHOLD:
                    per_player_data[str(custom_id)].positions.append(player_position)

            # Aggregate damage for all players (same tick)
            damage_at_tick = parsed_match.damage[tick]

            for player in parsed_match.players_data:
                custom_id = player.custom_id
                damage_by_player = damage_at_tick.get(custom_id, None)

                if damage_by_player:
                    per_player_data[custom_id].damage.append(damage_by_player)
                else:
                    per_player_data[custom_id].damage.append({})

        return per_player_data
