import uuid

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user, require_role
from app.core.database import get_db
from app.models.alias import ModelAlias
from app.models.user import User
from app.schemas.alias import (
    AliasCreateRequest,
    AliasListResponse,
    AliasResponse,
    AliasUpdateRequest,
)

router = APIRouter(prefix="/admin/model-aliases", tags=["Model Alias Management"])


def _alias_to_response(a: ModelAlias) -> AliasResponse:
    """Convert SQLAlchemy ModelAlias to AliasResponse with UUID→str."""
    return AliasResponse(
        id=str(a.id),
        alias_name=a.alias_name,
        target_model=a.target_model,
        description=a.description,
        is_active=a.is_active,
        created_at=a.created_at,
    )


@router.get("", response_model=AliasListResponse)
async def list_aliases(
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """List all model aliases."""
    result = await db.execute(
        select(ModelAlias).order_by(ModelAlias.alias_name)
    )
    aliases = result.scalars().all()
    return AliasListResponse(
        items=[_alias_to_response(a) for a in aliases]
    )


@router.post("", response_model=AliasResponse, status_code=status.HTTP_201_CREATED)
async def create_alias(
    body: AliasCreateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Create a new model alias."""
    # Check uniqueness
    result = await db.execute(
        select(ModelAlias).where(ModelAlias.alias_name == body.alias_name)
    )
    if result.scalar_one_or_none():
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail=f"Alias '{body.alias_name}' already exists",
        )

    alias = ModelAlias(
        id=uuid.uuid4(),
        alias_name=body.alias_name,
        target_model=body.target_model,
        description=body.description,
        is_active=body.is_active,
    )
    db.add(alias)
    await db.flush()
    return _alias_to_response(alias)


@router.put("/{alias_id}", response_model=AliasResponse)
async def update_alias(
    alias_id: uuid.UUID,
    body: AliasUpdateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Update a model alias."""
    result = await db.execute(
        select(ModelAlias).where(ModelAlias.id == alias_id)
    )
    alias = result.scalar_one_or_none()
    if not alias:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Alias not found",
        )

    update_data = body.model_dump(exclude_none=True)
    # If alias_name is being changed, check uniqueness
    if "alias_name" in update_data:
        existing = await db.execute(
            select(ModelAlias).where(ModelAlias.alias_name == update_data["alias_name"])
        )
        if existing.scalar_one_or_none():
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail=f"Alias '{update_data['alias_name']}' already exists",
            )

    for key, value in update_data.items():
        if hasattr(alias, key):
            setattr(alias, key, value)

    await db.flush()
    return _alias_to_response(alias)


@router.delete("/{alias_id}")
async def delete_alias(
    alias_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["super_admin"])),
):
    """Delete a model alias."""
    result = await db.execute(
        select(ModelAlias).where(ModelAlias.id == alias_id)
    )
    alias = result.scalar_one_or_none()
    if not alias:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Alias not found",
        )
    await db.delete(alias)
    await db.flush()
    return {"message": "Alias deleted successfully"}
