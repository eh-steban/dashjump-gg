from typing import Optional
from sqlmodel import SQLModel
from app.domain.boss import BossData
from app.domain.creep import LaneCreepData
from app.domain.deadlock_api import MatchMetadata
from app.domain.lane_pressure import LanePressureData
from app.domain.player import (
    PlayerData,
    # PlayerInfo,
    NPC,
    PlayerMatchData,
    Positions,
    ParsedAttackerVictimMap
)

class ParsedMatchResponse(SQLModel):
    match_duration_s: int
    match_start_time_s: int
    # Shape of damage data from parser:
    # Vec<HashMap<i32, HashMap<i32, Vec<DamageRecord>>>> (tick -> attacker -> victim -> Vec<DamageRecord>)
    # The parser puts i32 values as keys into our json, but they get converted to strings in Python
    # when parsed from JSON. So we use str keys here.
    #
    damage: list[ParsedAttackerVictimMap]
    players_data: list[PlayerData]
    # players: list[PlayerInfo]
    positions: Positions
    bosses: BossData
    lane_creep_data: LaneCreepData

# TODO: Created a temporary ParsedPlayer class to make this happy
# Might change this later
class TransformedMatchData(SQLModel):
    match_duration_s: int
    match_start_time_s: int
    players_data: list[PlayerData]
    # players: list[PlayerInfo]
    per_player_data: dict[str, PlayerMatchData]
    bosses: BossData
    lane_creep_data: LaneCreepData
    lane_pressure: LanePressureData = LanePressureData(pressure={})

class MatchAnalysis(SQLModel):
    match_metadata: MatchMetadata
    parsed_match_data: TransformedMatchData
