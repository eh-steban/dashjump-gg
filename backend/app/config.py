import os
from pydantic_settings import BaseSettings, SettingsConfigDict
from functools import lru_cache

@lru_cache
def get_settings():
    return Settings()

class Settings(BaseSettings):
    # environment: str = os.getenv("ENVIRONMENT", "development")
    JWT_SECRET_KEY: str = "secretKey"
    JWT_ALGORITHM: str = "algo"
    JWT_ACCESS_TOKEN_EXPIRE_MINUTES: int = 60
    FERNET_SECRET_KEY: str = "key"

    POSTGRES_USER: str = "user"
    POSTGRES_PASSWORD: str = "password"
    POSTGRES_DB: str = "db"
    DATABASE_URL: str = "url"
    TEST_DATABASE_URL: str = "url"

    DEADLOCK_API_KEY: str = "key"
    DEADLOCK_API_DOMAIN: str = "apiDomain"

    FRONTEND_BASE_URL: str = "url"
    BACKEND_BASE_URL: str = "url"
    PARSER_BASE_URL: str = "url"

    STEAM_WEB_API_KEY: str = "key"
    STEAM_HASH_SALT: str = "salt"

    model_config = SettingsConfigDict(env_file=".env", case_sensitive=True)