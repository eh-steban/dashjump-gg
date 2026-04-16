from sqlmodel import SQLModel


class HealthSample(SQLModel):
    time_s: float
    health: int


class MidBossSpawnEvent(SQLModel):
    spawn_cycle: int
    spawn_time_s: float


class MidBossKillEvent(SQLModel):
    spawn_cycle: int
    team_killed: int
    team_claimed: int
    rejuvs_by_team: dict[str, int]
    matchtime_s: float
    x: float
    y: float
    z: float
    bosses_remaining: int


class RejuvStatusEvent(SQLModel):
    matchtime_s: float
    player_pawn: int
    user_team: int
    killing_team: int
    event_type: int


class FightWindow(SQLModel):
    spawn_cycle: int
    window_start_s: float
    window_end_s: float
    health_at_start: int
    health_at_end: int
    health_samples: list[HealthSample]


class MidBossPostMatch(SQLModel):
    team_killed: int
    rejuvs_by_team: dict[str, int]
    destroyed_time_s: int


class MidBossData(SQLModel):
    boss_name_hash: str
    max_health: int | None = None
    spawn_events: list[MidBossSpawnEvent] = []
    kill_events: list[MidBossKillEvent] = []
    rejuv_events: list[RejuvStatusEvent] = []
    fight_windows: list[FightWindow] = []
    post_match: list[MidBossPostMatch] = []
