from typing import Annotated, Optional
from fastapi.params import Depends
from sqlmodel import select
from sqlalchemy.exc import IntegrityError, SQLAlchemyError
from sqlalchemy.ext.asyncio import AsyncSession
from app.domain.match_analysis import TransformedMatchData
from app.infra.db.parsed_match import ParsedMatch
from app.infra.db.session import get_db_session
from app.domain.exceptions import (
    MatchDataUnavailableException,
    MatchParseException,
    MatchDataIntegrityException,
)
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
        try:
            parsed_match = ParsedMatch(
                match_id=match_id,
                schema_version=schema_version,
                raw_payload_gzip=raw_payload_gzip,
                match_data=match_data,
                etag=etag,
            )
            session.add(parsed_match)
            await session.commit()
            await session.refresh(parsed_match)
        except IntegrityError as e:
            pgcode = getattr(getattr(e, "orig", None), "pgcode", None)
            if pgcode == "23505":
                await session.rollback()
                logger.warning(
                    "Match %s already exists in DB (concurrent insert), skipping", match_id
                )
            else:
                raise
        except SQLAlchemyError as e:
            # Prefer DBAPI message if present; otherwise first arg or class name
            minimal = getattr(e, "orig", None)
            if minimal is None:
                minimal = e.args[0] if e.args else e.__class__.__name__
            logger.error("Create parsed match failed: %s", minimal)
