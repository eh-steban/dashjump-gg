from typing import Annotated, Optional
from fastapi.params import Depends
from sqlmodel import select
from sqlalchemy.exc import IntegrityError, SQLAlchemyError
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.dialects.postgresql import insert as pg_insert
from app.domain.match_analysis import TransformedMatchData
from app.infra.db.parsed_match import ParsedMatch
from app.infra.db.session import get_db_session
from app.domain.exceptions import (
    MatchDataUnavailableException,
    MatchParseException,
    MatchDataIntegrityException,
)
from app.utils.datetime_utils import utcnow
from app.utils.logger import get_logger

logger = get_logger(__name__)

class ParsedMatchesRepo:
    async_session: Annotated[AsyncSession, Depends(get_db_session)]

    async def get_match_data(
        self,
        match_id: int,
        schema_version: int,
        session: Annotated[AsyncSession, Depends(get_db_session)],
    ) -> TransformedMatchData | None:
        try:
            logger.info(f"Fetching match_data for match_id={match_id}, schema_version={schema_version}")

            stmt = select(ParsedMatch).where(
                ParsedMatch.match_id == match_id,
                ParsedMatch.schema_version == schema_version,
            )
            result = await session.execute(stmt)
            record = result.scalar_one_or_none()

            if record is None:
                return None

            return TransformedMatchData(**record.match_data)

        except SQLAlchemyError as e:
            raise MatchDataIntegrityException(f"Fetch match_data failed: {e}")

    async def get_raw_gzip(
        self,
        match_id: int,
        schema_version: int,
        session: Annotated[AsyncSession, Depends(get_db_session)],
    ) -> Optional[bytes]:
        try:
            stmt = select(ParsedMatch.raw_payload_gzip).where(
                ParsedMatch.match_id == match_id,
                ParsedMatch.schema_version == schema_version,
            )
            result = await session.execute(stmt)
            return result.scalar_one_or_none()
        except SQLAlchemyError as e:
            raise MatchDataIntegrityException(f"Fetch raw_payload_gzip failed: {e}")

    async def create_parsed_match(
        self,
        match_id: int,
        schema_version: int,
        raw_payload_gzip: bytes,
        match_data: dict,
        etag: str,
        session: Annotated[AsyncSession, Depends(get_db_session)],
    ) -> None:
        # Upsert: insert on first parse, overwrite on re-parse (e.g. after a parser fix).
        # ON CONFLICT DO UPDATE targets the PK index. Concurrent racing inserts can
        # still surface a UniqueViolation on the overlapping composite constraint
        # `uq_match_id_schema_version` -- PG's arbiter check isn't deterministic when
        # two unique indexes overlap, and whichever fires first wins. We treat that
        # IntegrityError as "another request already persisted the row" (both races
        # produce identical parse output, so either one wins safely).
        try:
            now = utcnow()
            stmt = pg_insert(ParsedMatch).values(
                match_id=match_id,
                schema_version=schema_version,
                raw_payload_gzip=raw_payload_gzip,
                match_data=match_data,
                etag=etag,
                created_at=now,
                updated_at=now,
            )
            stmt = stmt.on_conflict_do_update(
                index_elements=["match_id"],
                set_={
                    "schema_version": stmt.excluded.schema_version,
                    "raw_payload_gzip": stmt.excluded.raw_payload_gzip,
                    "match_data": stmt.excluded.match_data,
                    "etag": stmt.excluded.etag,
                    "updated_at": stmt.excluded.updated_at,
                },
            )
            await session.execute(stmt)
            await session.commit()
        except IntegrityError as e:
            # Concurrent insert race: another request committed the same row while
            # our speculative insert was in flight. The row is now in the DB with
            # equivalent data, so the caller's next read will hit the cache.
            await session.rollback()
            minimal = getattr(e, "orig", None) or e.__class__.__name__
            logger.info(
                "Concurrent upsert race for match %s (another request persisted first): %s",
                match_id,
                minimal,
            )
        except SQLAlchemyError as e:
            await session.rollback()
            minimal = getattr(e, "orig", None)
            if minimal is None:
                minimal = e.args[0] if e.args else e.__class__.__name__
            logger.error("Upsert parsed match failed: %s", minimal)
            raise MatchDataIntegrityException(f"Failed to store match {match_id}")
