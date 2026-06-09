import time
from collections.abc import AsyncGenerator
from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Request, status
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user_from_api_token
from app.core.database import get_db
from app.models.user import User
from app.schemas.gateway import (
    ChatCompletionRequest,
    ChatCompletionResponse,
    ChatCompletionChunk,
    ChatChoice,
    ChatUsage,
    ModelInfo,
    ModelListResponse,
)
from app.services.gateway_service import gateway_service

router = APIRouter(tags=["AI Gateway (OpenAI Compatible)"])


@router.post("/v1/chat/completions")
async def chat_completions(
    request: Request,
    body: ChatCompletionRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user_from_api_token),
):
    """OpenAI-compatible chat completion endpoint.

    Supports both streaming (SSE) and non-streaming responses.
    Compatible with OpenAI SDK, Cursor, Cherry Studio, etc.
    """
    try:
        messages_dict = [m.model_dump() for m in body.messages]
        kwargs: dict[str, Any] = {
            "temperature": body.temperature,
            "max_tokens": body.max_tokens,
            "top_p": body.top_p,
            "frequency_penalty": body.frequency_penalty,
            "presence_penalty": body.presence_penalty,
            "stop": body.stop,
        }

        result = await gateway_service.chat_completion(
            db=db,
            user=current_user,
            model=body.model,
            messages=messages_dict,
            stream=body.stream,
            **kwargs,
        )

        if body.stream:
            return StreamingResponse(
                content=result,
                media_type="text/event-stream",
                headers={
                    "Cache-Control": "no-cache",
                    "Connection": "keep-alive",
                    "X-Accel-Buffering": "no",
                },
            )
        else:
            return result

    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=str(e),
        )


@router.post("/v1/embeddings")
async def embeddings(
    request: Request,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user_from_api_token),
):
    """OpenAI-compatible embeddings endpoint (proxy to provider)."""
    import json

    from app.core.security import decrypt_value
    from app.models.provider import Provider

    body = await request.json()
    model = body.get("model", "text-embedding-ada-002")

    # Find provider for embedding model
    result = await db.execute(
        """
        SELECT p.base_url, p.api_key_encrypted
        FROM providers p
        WHERE p.is_active = true AND p.models @> :model
        ORDER BY p.priority LIMIT 1
        """,
        {"model": json.dumps([model])},
    )
    row = result.first()
    if not row:
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=f"No active provider found for model: {model}",
        )

    from app.core.config import get_settings

    settings = get_settings()
    api_key = decrypt_value(row.api_key_encrypted, settings.ENCRYPTION_KEY)
    base_url = row.base_url.rstrip("/")

    import httpx

    async with httpx.AsyncClient() as client:
        resp = await client.post(
            f"{base_url}/v1/embeddings",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            json=body,
        )
        if resp.status_code != 200:
            raise HTTPException(
                status_code=status.HTTP_502_BAD_GATEWAY,
                detail=f"Provider error: {resp.text[:500]}",
            )
        return resp.json()


@router.get("/v1/models", response_model=ModelListResponse)
async def list_models(
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user_from_api_token),
):
    """OpenAI-compatible models list endpoint.
    
    Regular users see only their allowed model aliases.
    Admin users see all real models from providers.
    """
    models = await gateway_service.list_models(db, user=current_user)
    now = int(time.time())
    return ModelListResponse(
        data=[
            ModelInfo(id=m, created=now, owned_by="system") for m in models
        ]
    )


# Need to import StreamingResponse from FastAPI
from fastapi.responses import StreamingResponse
