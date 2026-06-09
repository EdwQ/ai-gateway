"""Rate limiting middleware."""

from collections.abc import Callable
from typing import Optional

from fastapi import HTTPException, Request, Response, status
from redis.asyncio import Redis
from starlette.middleware.base import BaseHTTPMiddleware

from app.core.config import get_settings

settings = get_settings()


class RateLimitMiddleware(BaseHTTPMiddleware):
    """Rate limiting middleware using Redis sliding window."""

    def __init__(self, app, redis_pool=None):
        super().__init__(app)
        self.redis_pool = redis_pool

    async def dispatch(self, request: Request, call_next: Callable) -> Response:
        # Skip rate limiting for health checks and static files
        if request.url.path in ("/health/liveness", "/health/readiness"):
            return await call_next(request)

        # Get client IP
        client_ip = request.client.host if request.client else "unknown"

        # Get user identifier from token if available
        user_key = client_ip
        auth_header = request.headers.get("Authorization", "")
        if auth_header.startswith("Bearer "):
            # Use token prefix as identifier (rate limit by token, not IP)
            token_pref = auth_header[7:27]  # First 20 chars of token
            if token_pref:
                user_key = f"user:{token_pref}"

        # Check rate limit
        try:
            from app.core.redis import redis_client

            if redis_client:
                key = f"ratelimit:{user_key}"
                current = await redis_client.incr(key)
                if current == 1:
                    await redis_client.expire(key, 1)  # 1 second window

                if current > settings.RATE_LIMIT_USER_QPS:
                    raise HTTPException(
                        status_code=status.HTTP_429_TOO_MANY_REQUESTS,
                        detail="Rate limit exceeded. Please slow down.",
                    )
        except HTTPException:
            raise
        except Exception:
            # If Redis is down, allow the request through
            pass

        return await call_next(request)
