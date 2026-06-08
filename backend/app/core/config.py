from pydantic_settings import BaseSettings, SettingsConfigDict
from functools import lru_cache


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
    )

    # App
    APP_NAME: str = "AI Gateway"
    DEBUG: bool = False
    SECRET_KEY: str = "change-this-to-a-random-secret-key-min-32-chars"
    ENCRYPTION_KEY: str = "change-this-to-32-byte-key!!"
    ALLOWED_ORIGINS: str = "http://localhost:3000,http://localhost:5173"

    # Database
    DATABASE_URL: str = "postgresql+asyncpg://postgres:postgres@localhost:5432/ai_gateway"
    DATABASE_URL_SYNC: str = "postgresql://postgres:postgres@localhost:5432/ai_gateway"

    # Redis
    REDIS_URL: str = "redis://localhost:6379/0"

    # DingTalk
    DINGTALK_CORP_ID: str = ""
    DINGTALK_APP_ID: str = ""
    DINGTALK_APP_SECRET: str = ""
    DINGTALK_AGENT_ID: str = ""

    # Frontend URL (for DingTalk OAuth redirect back)
    FRONTEND_URL: str = "http://localhost:3000"

    # JWT
    JWT_ACCESS_TOKEN_EXPIRE_MINUTES: int = 30
    JWT_REFRESH_TOKEN_EXPIRE_DAYS: int = 7

    # Quota
    DEFAULT_QUOTA_AMOUNT: float = 50.0

    # Prompt Audit
    PROMPT_SAVE_MODE: str = "off"

    # Rate Limit
    RATE_LIMIT_USER_QPS: int = 10
    RATE_LIMIT_PROVIDER_QPS: int = 100

    @property
    def ALLOWED_ORIGINS_LIST(self) -> list[str]:
        return [o.strip() for o in self.ALLOWED_ORIGINS.split(",") if o.strip()]

    @property
    def JWT_ALGORITHM(self) -> str:
        return "HS256"

    @property
    def JWT_ACCESS_TOKEN_EXPIRE_DELTA(self):
        from datetime import timedelta
        return timedelta(minutes=self.JWT_ACCESS_TOKEN_EXPIRE_MINUTES)

    @property
    def JWT_REFRESH_TOKEN_EXPIRE_DELTA(self):
        from datetime import timedelta
        return timedelta(days=self.JWT_REFRESH_TOKEN_EXPIRE_DAYS)


@lru_cache()
def get_settings() -> Settings:
    return Settings()
