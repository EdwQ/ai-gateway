import uuid
from datetime import datetime, timezone
from decimal import Decimal
from typing import Any

from redis.asyncio import Redis
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.config import get_settings
from app.core.security import (
    create_access_token,
    create_refresh_token,
    decode_token,
    get_token_blacklist_key,
)
from app.models.user import User
from app.services.dingtalk_service import dingtalk_service

settings = get_settings()


class AuthService:
    """Authentication service handling DingTalk OAuth and JWT."""

    async def login_via_dingtalk(
        self, auth_code: str, db: AsyncSession, redis: Redis
    ) -> dict[str, Any]:
        """Login or register via DingTalk auth code."""
        # Get user info from DingTalk
        dt_user = await dingtalk_service.get_user_info(auth_code)

        union_id = dt_user.get("unionid")
        if not union_id:
            raise ValueError("Failed to get DingTalk unionId")

        # Check if user exists
        result = await db.execute(select(User).where(User.union_id == union_id))
        user = result.scalar_one_or_none()

        # Auto-register if first login
        if user is None:
            user = User(
                id=uuid.uuid4(),
                union_id=union_id,
                user_id=dt_user.get("userid"),
                name=dt_user.get("name", "Unknown"),
                email=dt_user.get("email"),
                avatar=dt_user.get("avatar"),
                department_id=str(dt_user.get("dept_id_list", [None])[0])
                if dt_user.get("dept_id_list")
                else None,
                department_name=await self._get_dept_name(dt_user),
                title=dt_user.get("title"),
                role="employee",
                is_active=True,
                quota_balance=Decimal(str(settings.DEFAULT_QUOTA_AMOUNT)),
                quota_used=Decimal("0"),
                last_login_at=datetime.now(timezone.utc),
            )
            db.add(user)
        else:
            if not user.is_active:
                raise ValueError("User account is disabled")
            user.last_login_at = datetime.now(timezone.utc)

        await db.flush()

        # Generate JWT tokens
        access_token = create_access_token(
            data={"sub": str(user.id), "role": user.role}
        )
        refresh_token = create_refresh_token(
            data={"sub": str(user.id)}
        )

        return {
            "access_token": access_token,
            "refresh_token": refresh_token,
            "user": user,
        }

    async def refresh_access_token(
        self, refresh_token: str, redis: Redis
    ) -> dict[str, Any]:
        """Refresh access token using refresh token."""
        payload = decode_token(refresh_token)

        if payload.get("type") != "refresh":
            raise ValueError("Invalid refresh token type")

        # Check blacklist
        jti = payload.get("jti")
        if jti:
            blacklisted = await redis.get(get_token_blacklist_key(jti))
            if blacklisted:
                raise ValueError("Refresh token has been revoked")

        # Issue new access token
        new_access_token = create_access_token(
            data={"sub": payload["sub"], "role": payload.get("role", "employee")}
        )

        return {"access_token": new_access_token}

    async def logout(self, access_token: str, redis: Redis) -> None:
        """Logout: blacklist the JWT."""
        payload = decode_token(access_token)
        jti = payload.get("jti")
        exp = payload.get("exp")

        if jti and exp:
            ttl = exp - datetime.now(timezone.utc).timestamp()
            if ttl > 0:
                await redis.set(
                    get_token_blacklist_key(jti),
                    "1",
                    ex=int(ttl),
                )

    async def _get_dept_name(self, dt_user: dict) -> str | None:
        """Get department name from DingTalk response."""
        try:
            dept_ids = dt_user.get("dept_id_list", [])
            dept_names = dt_user.get("dept_name_list", "").split(",")
            if dept_names and dept_names[0]:
                return dept_names[0].strip()
            # Fallback to API call
            if dept_ids:
                detail = await dingtalk_service.get_department_detail(str(dept_ids[0]))
                return detail.get("name")
        except Exception:
            pass
        return None


auth_service = AuthService()
