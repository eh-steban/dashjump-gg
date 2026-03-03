"""Lane pressure calculation service."""

from typing import Optional

from app.domain.creep import CreepWaveData, CreepWaveSnapshot
from app.domain.lane_pressure import LanePressureData, LanePressureSnapshot
from app.domain.player import PlayerPosition, Positions
from app.utils.logger import get_logger

logger = get_logger(__name__)

# World bounds from parser's minimap constants
# TODO: Pull these values from parser. These rarely if
# ever change, but I haven't tested it either. Worth
# looking into this.
WORLD_MIN = -10752.0
WORLD_MAX = 10752.0
WORLD_SPAN = 21504.0  # WORLD_MAX - WORLD_MIN


class LanePressureCalculator:
    """Calculate lane pressure from creep wave positions."""

    @staticmethod
    def calculate_pressure(wave: CreepWaveSnapshot) -> float:
        """Calculate pressure (0-1) based on wave position.

        Pressure represents how far into enemy territory the wave has pushed.
        - Amber (team=2) base is at low Y; creeps push toward high Y (Sapphire base): pressure = normalized_y
        - Sapphire (team=3) base is at high Y; creeps push toward low Y (Amber base): pressure = 1.0 - normalized_y

        Args:
            wave: CreepWaveSnapshot with x, y, team

        Returns:
            float between 0.0 (own base) and 1.0 (enemy base)
        """
        normalized_y = (wave.y - WORLD_MIN) / WORLD_SPAN

        if wave.team == 2:  # Amber base at low Y, pushes toward high Y
            return normalized_y
        else:  # Sapphire base at high Y, pushes toward low Y
            return 1.0 - normalized_y

    @staticmethod
    def attribute_players(
        wave: CreepWaveSnapshot,
        player_positions: list[Optional[PlayerPosition]],
        proximity_threshold: float = 1500.0,
    ) -> list[int]:
        """Find players within proximity of wave centroid.

        Args:
            wave: CreepWaveSnapshot with x, y coordinates
            player_positions: List of PlayerPosition for current tick (may contain None)
            proximity_threshold: Distance threshold in world units (default 1500)

        Returns:
            List of player custom_ids (as int) within threshold
        """
        attributed = []

        for player_pos in player_positions:
            if player_pos is None:
                continue

            # Skip NPCs - only attribute to actual players
            if player_pos.is_npc:
                continue

            # Calculate Euclidean distance
            dx = player_pos.x - wave.x
            dy = player_pos.y - wave.y
            distance = (dx * dx + dy * dy) ** 0.5

            if distance <= proximity_threshold:
                # Convert custom_id to int
                attributed.append(int(player_pos.custom_id))

        return attributed

    @staticmethod
    def process_creep_waves(
        creep_waves: CreepWaveData,
        positions: Positions,
    ) -> LanePressureData:
        """Process all creep waves and calculate lane pressure.

        Args:
            creep_waves: CreepWaveData from parser
            positions: Player positions indexed by tick (list of PositionWindow)

        Returns:
            LanePressureData with pressure snapshots
        """
        pressure_timeline: dict[str, list[Optional[LanePressureSnapshot]]] = {}

        if not creep_waves or not creep_waves.waves:
            logger.warning("No creep wave data available - returning empty pressure data")
            return LanePressureData(pressure={})

        # Process each lane_team combination
        for lane_team_key, wave_snapshots in creep_waves.waves.items():
            pressure_snapshots: list[Optional[LanePressureSnapshot]] = []

            for tick_idx, wave in enumerate(wave_snapshots):
                if wave is None:
                    pressure_snapshots.append(None)
                    continue

                # Calculate pressure
                pressure = LanePressureCalculator.calculate_pressure(wave)

                # Get player positions for this tick
                player_positions = positions[tick_idx] if tick_idx < len(positions) else []

                # Attribute players
                attributed_players = LanePressureCalculator.attribute_players(
                    wave, player_positions
                )

                # Create pressure snapshot
                pressure_snapshot = LanePressureSnapshot(
                    pressure=pressure,
                    team=wave.team,
                    attributed_players=attributed_players,
                    wave_x=wave.x,
                    wave_y=wave.y,
                    wave_count=wave.count,
                )

                pressure_snapshots.append(pressure_snapshot)

            pressure_timeline[lane_team_key] = pressure_snapshots

        logger.debug(
            "Calculated pressure for %d lane/team combinations",
            len(pressure_timeline),
        )

        return LanePressureData(pressure=pressure_timeline)
