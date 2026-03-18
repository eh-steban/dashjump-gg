from typing import Optional

from sqlmodel import SQLModel


# PlayerInfo is pulled from Deadlock API (match_metadata call) and account_id
# is a 32-bit steam_id. The regular/web steam_id is 64-bit.
# player_slot is the slot in the match, starting from 0.
class PlayerInfo(SQLModel):
    # entity_id: str  # Ensure this stays a string for TransformService
    # custom_id: str
    # name: str
    # steam_id_32: int
    # lobby_player_slot: int
    team: int
    hero_id: int
    # lane: int


# TODO: Might not be needed anymore...
class NPC(SQLModel):
    entity_id: str  # Ensure this stays a string for TransformService
    name: str


class PlayerData(SQLModel):
    entity_id: str
    custom_id: str
    name: str
    steam_id_32: Optional[int] = None
    hero_id: Optional[int] = None
    lobby_player_slot: Optional[int] = None
    team: int
    lane: int
    # player_info: PlayerInfo


class DamageRecord(SQLModel):
    damage: Optional[int] = None
    pre_damage: Optional[int] = None
    type: Optional[int] = None
    citadel_type: Optional[int] = None
    entindex_inflictor: Optional[int] = None
    entindex_ability: Optional[int] = None
    damage_absorbed: Optional[int] = None
    victim_health_max: Optional[int] = None
    victim_health_new: Optional[int] = None
    flags: Optional[int] = None
    ability_id: Optional[int] = None
    attacker_class: Optional[int] = None
    victim_class: Optional[int] = None
    victim_shield_max: Optional[int] = None
    victim_shield_new: Optional[int] = None
    hits: Optional[int] = None
    health_lost: Optional[int] = None


DamageWindow = dict[str, list[DamageRecord]]
Damage = list[DamageWindow]
ParsedVictimDamage = DamageWindow
ParsedAttackerVictimMap = dict[str, ParsedVictimDamage]


class PlayerPosition(SQLModel):
    custom_id: str
    x: float
    y: float
    z: float
    is_npc: bool


PositionWindow = list[PlayerPosition]
Positions = list[PositionWindow]


# TODO: Need to include Health, Combat Type, and Move Type
class PlayerMatchData(SQLModel):
    positions: PositionWindow
    damage: Damage
