from fastapi import FastAPI
from app.utils.logger import LoggerManager
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.gzip import GZipMiddleware
# from app.api import auth, internal
# Only need the line below for now. Uncomment the line above
# when we implement internal API endpoints.
from app.api import auth, account, match, users, replay, session
from app.config import get_settings

app = FastAPI()
# Initialize logger manager (singleton)
LoggerManager()

settings = get_settings()

# Configure CORS to work w/ React frontend
origins = [settings.FRONTEND_BASE_URL]

app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    # Keeping these next 2 lines for initial development, can be restricted later
    allow_methods=["*"],
    allow_headers=["*"],
)
app.add_middleware(GZipMiddleware, minimum_size=500, compresslevel=9)

# Mount routers
# app.include_router(internal.router, prefix="/internal", tags=["Internal"])
app.include_router(auth.router, prefix="/auth")
app.include_router(users.router, prefix="/users")
app.include_router(account.router, prefix="/account")
app.include_router(match.router, prefix="/match")
app.include_router(replay.router, prefix="/replay")
app.include_router(session.router, prefix="/session")
