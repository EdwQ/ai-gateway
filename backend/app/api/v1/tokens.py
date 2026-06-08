import uuid

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user
from app.core.database import get_db
from app.models.user import User
from app.schemas.token import (
    ApiTokenCreatedResponse,
    ApiTokenCreateRequest,
    ApiTokenListResponse,
    ApiTokenResponse,
    ApiTokenRotateResponse,
)
from app.services.token_service import token_service
from app.services.audit_service import audit_service

router = APIRouter(prefix="/tokens", tags=["API Token Management"])


@router.get("", response_model=ApiTokenListResponse)
async def list_tokens(
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List current user's API tokens."""
    tokens = await token_service.list_tokens(db, current_user.id)
    return ApiTokenListResponse(
        items=[
            ApiTokenResponse(
                id=str(t.id),
                token_prefix=t.token_prefix,
                name=t.name,
                is_active=t.is_active,
                last_used_at=t.last_used_at,
                expires_at=t.expires_at,
                created_at=t.created_at,
            )
            for t in tokens
        ]
    )


@router.post("", response_model=ApiTokenCreatedResponse, status_code=status.HTTP_201_CREATED)
async def create_token(
    body: ApiTokenCreateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Create a new API token."""
    try:
        token, raw_token = await token_service.create_token(
            db, current_user.id, body.name
        )
        return ApiTokenCreatedResponse(
            id=str(token.id),
            token=raw_token,
            name=token.name,
            created_at=token.created_at,
        )
    except ValueError as e:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(e))


@router.delete("/{token_id}", response_model=ApiTokenResponse)
async def delete_token(
    token_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Deactivate/delete a token."""
    token = await token_service.deactivate_token(db, token_id, current_user.id)
    if not token:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Token not found",
        )
    return ApiTokenResponse(
        id=str(token.id),
        token_prefix=token.token_prefix,
        name=token.name,
        is_active=token.is_active,
        last_used_at=token.last_used_at,
        expires_at=token.expires_at,
        created_at=token.created_at,
    )


@router.post("/{token_id}/rotate", response_model=ApiTokenRotateResponse)
async def rotate_token(
    token_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Rotate an API token (deactivate old, create new)."""
    result = await token_service.rotate_token(db, token_id, current_user.id)
    if not result:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Token not found",
        )
    new_token, raw_token = result
    return ApiTokenRotateResponse(
        id=str(new_token.id),
        token=raw_token,
        name=new_token.name,
    )
