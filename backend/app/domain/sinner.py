from sqlmodel import SQLModel, Field
from typing import Optional


class SinnerSnapshot(SQLModel):
    entity_index: int
    x: float
    y: float
    z: float
    spawn_time_s: int
    max_health: int
    death_time_s: Optional[int] = None
    time_alive_s: Optional[int] = None
    killer_player_slot: Optional[int] = None
    # Serde serializes HashMap<u32, i32> keys as strings, so JSON keys arrive as strings.
    # dict[str, int] matches the wire format: {"0": 160, "4": 80}
    retaliation_damage: dict[str, int] = Field(default_factory=dict)
