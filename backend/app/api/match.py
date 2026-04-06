import json
import time
from typing import Annotated
from fastapi import (
    APIRouter,
    HTTPException,
    Request,
    Response,
    status,
    Depends
)
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.exc import SQLAlchemyError
from app.services.deadlock_api_service import DeadlockAPIService
from app.services.parser_service import ParserService
from app.repo.parsed_matches_repo import ParsedMatchesRepo
from app.domain.match_analysis import MatchAnalysis
from app.infra.db.session import get_db_session
from app.config import Settings, get_settings
from app.utils.http_cache import check_if_not_modified
from app.utils.logger import get_logger, fmt_duration
from app.domain.exceptions import (
    DeadlockAPIError,
    ParserServiceError,
    MatchDataUnavailableException,
    MatchDataIntegrityException,
)
from app.application.use_cases.analyze_match import AnalyzeMatchUseCase

router = APIRouter()
logger = get_logger(__name__)

SessionDep = Annotated[AsyncSession, Depends(get_db_session)]
SettingsDep = Annotated[Settings, Depends(get_settings)]

def get_deadlock_service() -> DeadlockAPIService:
    return DeadlockAPIService()

def get_parser_service() -> ParserService:
    return ParserService()

ServiceDep = Annotated[DeadlockAPIService, Depends(get_deadlock_service)]
ParserServiceDep = Annotated[ParserService, Depends(get_parser_service)]

@router.get("/analysis/{match_id}", response_model=MatchAnalysis)
async def get_match_analysis(
    request: Request,
    match_id: int,
    session: SessionDep,
    settings: SettingsDep,
    deadlock_api_service: ServiceDep,
    parser_service: ParserServiceDep,
):

    schema_version = 1
    repo = ParsedMatchesRepo()
    request_start = time.perf_counter()

    try:
        # Execute use case
        use_case = AnalyzeMatchUseCase(parser_service, deadlock_api_service, repo)
        match_data, etag = await use_case.execute(match_id, schema_version, session)

        # Check ETag for 304 Not Modified
        if request_etag := request.headers.get("If-None-Match"):
            not_modified_response = check_if_not_modified(request_etag, etag)
            if not_modified_response:
                return Response(
                    status_code=status.HTTP_304_NOT_MODIFIED,
                    headers={"ETag": etag, "Cache-Control": "public, max-age=300"},
                )

    except MatchDataUnavailableException:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Parsed payload missing in DB",
        )
    except MatchDataIntegrityException as e:
        logger.exception("Failed to store parsed payload in DB: %s", e)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to store parsed payload in DB",
        )
    except SQLAlchemyError as db_err:
        logger.exception(
            "Database error while fetching parsed payload for match_id=%s", match_id
        )
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Database error while fetching parsed payload",
        )
    except DeadlockAPIError as dl_api_err:
        logger.exception(
            "Deadlock API error occurred for match_id=%s. Error: %s",
            match_id,
            dl_api_err,
        )
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Deadlock API error occurred. Check logs for details.",
        )
    except ParserServiceError as parser_err:
        logger.exception(
            "Parser service error for match_id=%s: %s",
            match_id,
            parser_err,
        )
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail="Parser service unavailable. Please try again later.",
        )
    except HTTPException:
        raise
    except Exception as e:
        logger.exception(
            "Unhandled error while building match analysis for match_id=%s. Error: %s",
            match_id,
            e,
        )
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Internal Server Error",
        )

    match_metadata = await deadlock_api_service.get_match_metadata_for(match_id)

    # Prepare analysis
    # match_info = match_metadata.match_info
    # players_list = match_info.players
    analysis = MatchAnalysis(
        match_metadata=match_metadata,
        parsed_match_data=match_data,
    )

    response_content = json.dumps(analysis.model_dump())
    response = Response(
        content=response_content, media_type="application/json"
    )
    response.headers["ETag"] = etag
    response.headers["Cache-Control"] = "public, max-age=300"

    response_size = len(response_content.encode('utf-8'))
    match_time_minutes = analysis.parsed_match_data.match_duration_s / 60
    total_elapsed_ms = (time.perf_counter() - request_start) * 1000
    logger.info(
        f"Match analysis for match_id={match_id} served with ETag={etag}. "
        f"Match time={match_time_minutes:.2f} minutes ({analysis.parsed_match_data.match_duration_s}s). "
        f"Response size={response_size:,} bytes ({response_size / 1024:.2f} KB, {response_size / (1024 * 1024):.2f} MB). "
        f"Total latency={fmt_duration(total_elapsed_ms)}"
    )
    return response
