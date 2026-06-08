from collections.abc import AsyncGenerator
from typing import Optional

from redis.asyncio import Redis

from app.core.config import get_settings

settings = get_settings()

redis_client: Optional[Redis] = None


async def init_redis() -> Redis:
    """Initialize Redis connection pool."""
    global redis_client
    redis_client = Redis.from_url(
        settings.REDIS_URL,
        encoding="utf-8",
        decode_responses=True,
        socket_keepalive=True,
    )
    return redis_client


async def close_redis() -> None:
    """Close Redis connection."""
    global redis_client
    if redis_client:
        await redis_client.close()
        redis_client = None


async def get_redis() -> AsyncGenerator[Redis, None]:
    """FastAPI dependency for Redis client."""
    if redis_client is None:
        await init_redis()
    try:
        yield redis_client
    finally:
        pass
