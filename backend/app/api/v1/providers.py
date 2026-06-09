import uuid

import httpx
from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user, require_role
from app.core.config import get_settings
from app.core.database import get_db
from app.core.security import decrypt_value, encrypt_value
from app.models.provider import Provider, ProviderKey
from app.models.user import User
from app.schemas.provider import (
    HealthCheckResponse,
    ProviderCreateRequest,
    ProviderListResponse,
    ProviderResponse,
    ProviderUpdateRequest,
)

router = APIRouter(prefix="/admin/providers", tags=["Provider Management"])

settings = get_settings()


@router.get("", response_model=ProviderListResponse)
async def list_providers(
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """List all providers."""
    result = await db.execute(
        select(Provider).order_by(Provider.priority)
    )
    providers = result.scalars().all()
    return ProviderListResponse(
        items=[_provider_to_response(p) for p in providers]
    )


@router.post("", response_model=ProviderResponse, status_code=status.HTTP_201_CREATED)
async def create_provider(
    body: ProviderCreateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Create a new provider."""
    encrypted_key = encrypt_value(body.api_key, settings.ENCRYPTION_KEY)
    provider = Provider(
        id=uuid.uuid4(),
        name=body.name,
        display_name=body.display_name,
        base_url=body.base_url,
        api_key_encrypted=encrypted_key,
        models=body.models,
        is_active=body.is_active,
        priority=body.priority,
        rate_limit_qps=body.rate_limit_qps,
    )
    db.add(provider)
    await db.flush()
    return ProviderResponse(
        id=str(provider.id),
        name=provider.name,
        display_name=provider.display_name,
        base_url=provider.base_url,
        models=provider.models or [],
        is_active=provider.is_active,
        priority=provider.priority,
        health_status=provider.health_status,
        rate_limit_qps=provider.rate_limit_qps,
        created_at=provider.created_at,
        updated_at=provider.updated_at,
    )


def _provider_to_response(p: Provider) -> ProviderResponse:
    """Convert SQLAlchemy Provider model to ProviderResponse with UUID→str."""
    return ProviderResponse(
        id=str(p.id),
        name=p.name,
        display_name=p.display_name,
        base_url=p.base_url,
        models=p.models or [],
        is_active=p.is_active,
        priority=p.priority,
        health_status=p.health_status,
        rate_limit_qps=p.rate_limit_qps,
        created_at=p.created_at,
        updated_at=p.updated_at,
    )


@router.put("/{provider_id}", response_model=ProviderResponse)
async def update_provider(
    provider_id: uuid.UUID,
    body: ProviderUpdateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Update provider settings."""
    result = await db.execute(
        select(Provider).where(Provider.id == provider_id)
    )
    provider = result.scalar_one_or_none()
    if not provider:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Provider not found",
        )

    update_data = body.model_dump(exclude_none=True)
    if "api_key" in update_data:
        update_data["api_key_encrypted"] = encrypt_value(
            update_data.pop("api_key"), settings.ENCRYPTION_KEY
        )

    for key, value in update_data.items():
        if hasattr(provider, key):
            setattr(provider, key, value)

    await db.flush()
    return _provider_to_response(provider)


@router.delete("/{provider_id}")
async def delete_provider(
    provider_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["super_admin"])),
):
    """Delete a provider (super_admin only)."""
    result = await db.execute(
        select(Provider).where(Provider.id == provider_id)
    )
    provider = result.scalar_one_or_none()
    if not provider:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Provider not found",
        )
    await db.delete(provider)
    await db.flush()
    return {"message": "Provider deleted successfully"}


@router.post("/{provider_id}/check", response_model=HealthCheckResponse)
async def check_provider_health(
    provider_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Check provider health by calling its models endpoint."""
    result = await db.execute(
        select(Provider).where(Provider.id == provider_id)
    )
    provider = result.scalar_one_or_none()
    if not provider:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Provider not found",
        )

    import time

    api_key = decrypt_value(provider.api_key_encrypted, settings.ENCRYPTION_KEY)
    base_url = provider.base_url.rstrip("/")

    start = time.time()
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            resp = await client.get(
                f"{base_url}/v1/models",
                headers={"Authorization": f"Bearer {api_key}"},
            )
            latency = int((time.time() - start) * 1000)

            if resp.status_code == 200:
                provider.health_status = "healthy"
                status_text = "healthy"
            else:
                provider.health_status = "degraded"
                status_text = f"degraded (HTTP {resp.status_code})"

            await db.flush()
            return HealthCheckResponse(status=status_text, latency_ms=latency)

    except Exception as e:
        provider.health_status = "down"
        await db.flush()
        return HealthCheckResponse(
            status=f"down ({str(e)[:100]})",
            latency_ms=0,
        )


class DiscoverModelsRequest(BaseModel):
    base_url: str
    api_key: str


@router.post("/discover-models", response_model=dict)
async def discover_provider_models(
    body: DiscoverModelsRequest,
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Fetch available models from a provider's /v1/models endpoint."""
    import time

    base_url = body.base_url.rstrip("/")
    start = time.time()
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            resp = await client.get(
                f"{base_url}/v1/models",
                headers={"Authorization": f"Bearer {body.api_key}"},
            )
            latency = int((time.time() - start) * 1000)

            if resp.status_code == 200:
                data = resp.json()
                models = data.get("data", [])
                # Extract model IDs from various response formats
                # OpenAI format: {"data": [{"id": "gpt-4o", ...}, ...]}
                # Some providers return: {"data": ["model1", "model2"]}
                model_ids = []
                for m in models:
                    if isinstance(m, dict):
                        model_ids.append(m.get("id", ""))
                    elif isinstance(m, str):
                        model_ids.append(m)
                return {"models": [m for m in model_ids if m], "latency_ms": latency}
            else:
                return {"models": [], "latency_ms": latency, "error": f"HTTP {resp.status_code}"}

    except Exception as e:
        return {"models": [], "latency_ms": 0, "error": str(e)[:200]}
