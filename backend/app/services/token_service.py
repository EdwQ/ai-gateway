import uuid
from typing import Optional

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.security import generate_api_token, hash_token
from app.models.token import ApiToken
from app.models.user import User


class TokenService:
    """API Token management service."""

    MAX_TOKENS_PER_USER = 10

    async def create_token(
        self, db: AsyncSession, user_id: uuid.UUID, name: str = ""
    ) -> tuple[ApiToken, str]:
        """Create a new API token. Returns (token_obj, raw_token)."""
        # Check token limit
        result = await db.execute(
            select(ApiToken).where(
                ApiToken.user_id == user_id, ApiToken.is_active == True  # noqa: E712
            )
        )
        active_tokens = result.scalars().all()
        if len(active_tokens) >= self.MAX_TOKENS_PER_USER:
            raise ValueError(
                f"Maximum {self.MAX_TOKENS_PER_USER} active tokens allowed"
            )

        raw_token = generate_api_token()
        token_hash = hash_token(raw_token)
        token_prefix = raw_token[:20]  # e.g. "sk-company-a1b2c3d4e5"

        token = ApiToken(
            id=uuid.uuid4(),
            user_id=user_id,
            token_hash=token_hash,
            token_prefix=token_prefix,
            name=name,
        )
        db.add(token)
        await db.flush()

        return token, raw_token

    async def list_tokens(
        self, db: AsyncSession, user_id: uuid.UUID
    ) -> list[ApiToken]:
        """List all tokens for a user."""
        result = await db.execute(
            select(ApiToken)
            .where(ApiToken.user_id == user_id)
            .order_by(ApiToken.created_at.desc())
        )
        return list(result.scalars().all())

    async def deactivate_token(
        self, db: AsyncSession, token_id: uuid.UUID, user_id: uuid.UUID
    ) -> Optional[ApiToken]:
        """Deactivate a token."""
        result = await db.execute(
            select(ApiToken).where(
                ApiToken.id == token_id, ApiToken.user_id == user_id
            )
        )
        token = result.scalar_one_or_none()
        if not token:
            return None
        token.is_active = False
        await db.flush()
        return token

    async def rotate_token(
        self, db: AsyncSession, token_id: uuid.UUID, user_id: uuid.UUID
    ) -> Optional[tuple[ApiToken, str]]:
        """Rotate token: generate new one, deactivate old."""
        result = await db.execute(
            select(ApiToken).where(
                ApiToken.id == token_id, ApiToken.user_id == user_id
            )
        )
        old_token = result.scalar_one_or_none()
        if not old_token:
            return None

        # Deactivate old
        old_token.is_active = False

        # Create new
        new_token, raw_token = await self.create_token(db, user_id, old_token.name)
        return new_token, raw_token

    async def validate_token(
        self, db: AsyncSession, raw_token: str
    ) -> Optional[User]:
        """Validate API token and return user if valid."""
        from app.core.security import verify_token_hash

        token_hash = hash_token(raw_token)
        result = await db.execute(
            select(ApiToken).where(
                ApiToken.token_hash == token_hash,
                ApiToken.is_active == True,  # noqa: E712
            )
        )
        token = result.scalar_one_or_none()
        if not token:
            return None

        # Get user
        user_result = await db.execute(
            select(User).where(
                User.id == token.user_id, User.is_active == True  # noqa: E712
            )
        )
        user = user_result.scalar_one_or_none()
        if not user:
            return None

        # Update last_used_at
        from datetime import datetime, timezone

        token.last_used_at = datetime.now(timezone.utc)
        await db.flush()

        return user


token_service = TokenService()
