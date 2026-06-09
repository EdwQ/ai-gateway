import time
from contextlib import asynccontextmanager
from typing import AsyncGenerator

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from prometheus_client import Counter, Histogram, generate_latest, REGISTRY

from app.core.config import get_settings
from app.core.database import init_db
from app.core.redis import init_redis, close_redis

settings = get_settings()

# Prometheus metrics
REQUEST_COUNT = Counter(
    "ai_gateway_requests_total", "Total requests", ["method", "endpoint", "status"]
)
REQUEST_LATENCY = Histogram(
    "ai_gateway_request_duration_ms", "Request latency in ms", ["method", "endpoint"]
)
SSE_STREAM_COUNT = Counter(
    "ai_gateway_sse_streams_total", "Total SSE streams", ["model"]
)
TOKEN_USAGE = Counter(
    "ai_gateway_tokens_total", "Total tokens processed", ["model", "type"]
)


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]:
    """Application lifespan: init resources on startup, cleanup on shutdown."""
    # Startup
    await init_redis()
    if settings.DEBUG:
        await init_db()
    yield
    # Shutdown
    await close_redis()


app = FastAPI(
    title=settings.APP_NAME,
    version="1.0.0",
    lifespan=lifespan,
    docs_url="/docs",
    redoc_url="/redoc",
)

# CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.ALLOWED_ORIGINS_LIST,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# Request metrics middleware
@app.middleware("http")
async def metrics_middleware(request: Request, call_next):
    start = time.time()
    response = await call_next(request)
    duration = int((time.time() - start) * 1000)

    REQUEST_COUNT.labels(
        method=request.method,
        endpoint=request.url.path,
        status=response.status_code,
    ).inc()
    REQUEST_LATENCY.labels(
        method=request.method,
        endpoint=request.url.path,
    ).observe(duration)

    return response


# Health check endpoints
@app.get("/health/liveness")
async def liveness():
    """Liveness probe for K8s."""
    return {"status": "alive", "timestamp": time.time()}


@app.get("/health/readiness")
async def readiness():
    """Readiness probe for K8s."""
    from app.core.database import engine
    from app.core.redis import redis_client

    # Check database
    try:
        from sqlalchemy import text
        async with engine.connect() as conn:
            await conn.execute(text("SELECT 1"))
        db_ok = True
    except Exception:
        db_ok = False

    # Check Redis
    redis_ok = False
    if redis_client:
        try:
            await redis_client.ping()
            redis_ok = True
        except Exception:
            redis_ok = False

    if not db_ok or not redis_ok:
        return JSONResponse(
            status_code=503,
            content={
                "status": "not ready",
                "database": "ok" if db_ok else "down",
                "redis": "ok" if redis_ok else "down",
            },
        )

    return {
        "status": "ready",
        "database": "ok",
        "redis": "ok",
        "timestamp": time.time(),
    }


# Prometheus metrics endpoint
@app.get("/metrics")
async def metrics():
    """Prometheus metrics endpoint."""
    from fastapi.responses import PlainTextResponse
    return PlainTextResponse(generate_latest(REGISTRY).decode("utf-8"))


# Import and register API routers
from app.api.v1 import auth, users, tokens, gateway, providers, stats, audit, aliases

app.include_router(auth.router, prefix="/api/v1")
app.include_router(users.router, prefix="/api/v1")
app.include_router(tokens.router, prefix="/api/v1")
app.include_router(gateway.router, prefix="")
app.include_router(providers.router, prefix="/api/v1")
app.include_router(stats.router, prefix="/api/v1")
app.include_router(audit.router, prefix="/api/v1")
app.include_router(aliases.router, prefix="/api/v1")


@app.get("/")
async def root():
    """Root endpoint."""
    return {
        "name": settings.APP_NAME,
        "version": "1.0.0",
        "docs": "/docs",
        "health": "/health/liveness",
    }


# Error handlers
@app.exception_handler(Exception)
async def global_exception_handler(request: Request, exc: Exception):
    """Global exception handler."""
    return JSONResponse(
        status_code=500,
        content={"detail": f"Internal server error: {str(exc)}"},
    )
