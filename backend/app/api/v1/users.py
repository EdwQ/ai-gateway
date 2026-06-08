import uuid

from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user, require_role
from app.core.database import get_db
from app.models.user import User
from app.schemas.user import UserListResponse, UserResponse, UserUpdateRequest
from app.services.user_service import user_service

router = APIRouter(prefix="/users", tags=["User Management"])


@router.get("", response_model=UserListResponse)
async def list_users(
    page: int = Query(1, ge=1),
    page_size: int = Query(20, ge=1, le=100),
    search: str = Query(None),
    is_active: bool = Query(None),
    role: str = Query(None),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "finance", "super_admin"])),
):
    """List users with pagination and filters."""
    result = await user_service.list_users(
        db, page=page, page_size=page_size,
        search=search, is_active=is_active, role=role,
    )
    return UserListResponse(
        items=[UserResponse.model_validate(u) for u in result["items"]],
        total=result["total"],
        page=result["page"],
        page_size=result["page_size"],
    )


@router.get("/{user_id}", response_model=UserResponse)
async def get_user(
    user_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Get user details."""
    user = await user_service.get_user(db, user_id)
    if not user:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")
    return UserResponse.model_validate(user)


@router.patch("/{user_id}", response_model=UserResponse)
async def update_user(
    user_id: uuid.UUID,
    body: UserUpdateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """Update user (role, status, quota, etc.)."""
    update_data = body.model_dump(exclude_none=True)
    user = await user_service.update_user(db, user_id, **update_data)
    if not user:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")
    return UserResponse.model_validate(user)


@router.delete("/{user_id}", response_model=UserResponse)
async def deactivate_user(
    user_id: uuid.UUID,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["super_admin"])),
):
    """Deactivate user (super_admin only)."""
    user = await user_service.deactivate_user(db, user_id)
    if not user:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")
    return UserResponse.model_validate(user)
