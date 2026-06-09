import uuid
from datetime import datetime
from decimal import Decimal
from typing import TYPE_CHECKING, Optional

from sqlalchemy import Boolean, DateTime, JSON, Numeric, String, func
from sqlalchemy.dialects.postgresql import UUID
from decimal import Decimal
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.models.base import Base, TimestampMixin

if TYPE_CHECKING:
    from app.models.token import ApiToken
    from app.models.usage import UsageLog


class User(Base, TimestampMixin):
    __tablename__ = "users"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    union_id: Mapped[str] = mapped_column(
        String(128), unique=True, nullable=False, index=True
    )
    user_id: Mapped[Optional[str]] = mapped_column(
        String(128), unique=True, index=True, nullable=True
    )
    name: Mapped[str] = mapped_column(String(128), nullable=False)
    email: Mapped[Optional[str]] = mapped_column(String(256), nullable=True)
    avatar: Mapped[Optional[str]] = mapped_column(String(512), nullable=True)
    department_id: Mapped[Optional[str]] = mapped_column(
        String(64), index=True, nullable=True
    )
    department_name: Mapped[Optional[str]] = mapped_column(
        String(256), nullable=True
    )
    title: Mapped[Optional[str]] = mapped_column(String(256), nullable=True)
    role: Mapped[str] = mapped_column(String(32), default="employee")
    is_active: Mapped[bool] = mapped_column(Boolean, default=True)
    quota_balance: Mapped[Decimal] = mapped_column(
        Numeric(12, 4), default=0
    )
    quota_used: Mapped[Decimal] = mapped_column(Numeric(12, 4), default=0)
    last_login_at: Mapped[Optional[datetime]] = mapped_column(
        DateTime(timezone=True), nullable=True
    )

    # Relationships
    api_tokens: Mapped[list["ApiToken"]] = relationship(
        "ApiToken", back_populates="user", cascade="all, delete-orphan"
    )
    usage_logs: Mapped[list["UsageLog"]] = relationship(
        "UsageLog", back_populates="user", cascade="all, delete-orphan"
    )

    def __repr__(self) -> str:
        return f"<User {self.name} ({self.role})>"
