"""Creep domain models for per-creep lane tracking."""

from typing import Optional
from sqlmodel import SQLModel


class CreepSnapshot(SQLModel):
    """Per-second snapshot for one individual creep while it is alive."""

    x: float
    y: float
    lane: int
    team: int
    wave_id: str  # "lane_team_spawnsec" e.g. "1_2_45"
    nearby_players: list[int]  # player custom_ids within 1500 world units


class WaveMeta(SQLModel):
    """Metadata computed across a wave's full lifetime.

    Wave membership is derived by filtering creep snapshots where wave_id matches --
    no entity indices stored here to avoid duplication.
    """

    lane: int
    team: int
    spawn_sec: int
    last_death_sec: Optional[int] = None   # match-relative second of last creep death
    last_death_x: Optional[float] = None
    last_death_y: Optional[float] = None


class LaneCreepData(SQLModel):
    """Container for all per-creep lane tracking data.

    creeps: str(entity_index) -> match-relative timeline (None = dead or not yet spawned)
    wave_meta: wave_id -> metadata
    """

    creeps: dict[str, list[Optional[CreepSnapshot]]]
    wave_meta: dict[str, WaveMeta]
