"""Lane pressure domain models for map control tracking."""

from typing import Optional
from sqlmodel import SQLModel


class LanePressureSnapshot(SQLModel):
    """Snapshot of lane pressure at a given second.

    Pressure represents how far into enemy territory the wave has pushed,
    scaled by alive creep count.
    - 0.0 = wave dead or at own base
    - >0.0 = wave alive; higher values mean closer to enemy objective with more creeps
    """

    pressure: float               # raw_pressure * (alive_count * 0.25)
    team: int                     # Team (2 = Amber, 3 = Sapphire)
    wave_id: str                  # "lane_team_spawnsec"
    creep_count: int              # alive creep count at this second
    attributed_players: list[int] # union of nearby_players across alive creeps


class LanePressureData(SQLModel):
    """Container for all lane pressure data.

    Key: wave_id (same keys as LaneCreepData.wave_meta)
    Value: per-second snapshots (None = no alive creeps that second)
    """

    pressure: dict[str, list[Optional[LanePressureSnapshot]]]
