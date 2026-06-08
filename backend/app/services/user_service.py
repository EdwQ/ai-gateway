import uuid
from decimal import Decimal
from typing import Optional

from sqlalchemy import select, func
from sqlalchemy.ext.asyncio import AsyncSession

from app.models.user import User


class UserService:
    """User management service."""

    async def list_users(
        self,
        db: AsyncSession,
        page: int = 1,
        page_size: int = 20,
        search: Optional[str] = None,
        is_active: Optional[bool] = None,
        role: Optional[str] = None,
    ) -> dict:
        """List users with pagination and filters."""
        query = select(User)

        if search:
            query = query.where(
                User.name.ilike(f"%{search}%")
                | User.email.ilike(f"%{search}%")
                | User.department_name.ilike(f"%{search}%")
            )
        if is_active is not None:
            query = query.where(User.is_active == is_active)
        if role:
            query = query.where(User.role == role)

        # Count total
        count_query = select(func.count()).select_from(query.subquery())
        total_result = await db.execute(count_query)
        total = total_result.scalar() or 0

        # Paginate
        offset = (page - 1) * page_size
        query = query.order_by(User.created_at.desc()).offset(offset).limit(page_size)
        result = await db.execute(query)
        users = result.scalars().all()

        return {
            "items": list(users),
            "total": total,
            "page": page,
            "page_size": page_size,
        }

    async def get_user(self, db: AsyncSession, user_id: uuid.UUID) -> Optional[User]:
        """Get user by ID."""
        result = await db.execute(select(User).where(User.id == user_id))
        return result.scalar_one_or_none()

    async def update_user(
        self,
        db: AsyncSession,
        user_id: uuid.UUID,
        **kwargs,
    ) -> Optional[User]:
        """Update user fields."""
        user = await self.get_user(db, user_id)
        if not user:
            return None

        for key, value in kwargs.items():
            if value is not None and hasattr(user, key):
                setattr(user, key, value)

        await db.flush()
        return user

    async def deactivate_user(
        self, db: AsyncSession, user_id: uuid.UUID
    ) -> Optional[User]:
        """Deactivate user."""
        return await self.update_user(db, user_id, is_active=False)

    async def activate_user(
        self, db: AsyncSession, user_id: uuid.UUID
    ) -> Optional[User]:
        """Activate user."""
        return await self.update_user(db, user_id, is_active=True)


user_service = UserService()
