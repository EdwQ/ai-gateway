import uuid
from datetime import datetime
from decimal import Decimal
from typing import TYPE_CHECKING, Optional

from sqlalchemy import (
    Boolean,
    DateTime,
    Numeric,
    ForeignKey,
    Integer,
    String,
    Text,
    func,
)
from sqlalchemy.dialects.postgresql import UUID
from decimal import Decimal
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.models.base import Base

if TYPE_CHECKING:
    from app.models.user import User


class UsageLog(Base):
    __tablename__ = "usage_logs"

    id: Mapped[int] = mapped_column(
        Integer, primary_key=True, autoincrement=True
    )
    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id"), nullable=False, index=True
    )
    token_id: Mapped[Optional[uuid.UUID]] = mapped_column(
        UUID(as_uuid=True), nullable=True
    )
    model: Mapped[str] = mapped_column(
        String(128), nullable=False, index=True
    )
    provider: Mapped[str] = mapped_column(String(64), nullable=False)
    prompt_tokens: Mapped[int] = mapped_column(Integer, default=0)
    completion_tokens: Mapped[int] = mapped_column(Integer, default=0)
    total_tokens: Mapped[int] = mapped_column(Integer, default=0)
    cost_rmb: Mapped[Decimal] = mapped_column(Numeric(12, 6), default=0)
    duration_ms: Mapped[int] = mapped_column(Integer, default=0)
    is_stream: Mapped[bool] = mapped_column(Boolean, default=False)
    is_success: Mapped[bool] = mapped_column(Boolean, default=True)
    status_code: Mapped[int] = mapped_column(Integer, default=200)
    error_message: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    ip_address: Mapped[Optional[str]] = mapped_column(
        String(45), nullable=True
    )
    request_id: Mapped[Optional[str]] = mapped_column(
        String(64), unique=True, index=True, nullable=True
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), index=True
    )

    # Relationships
    user: Mapped["User"] = relationship("User", back_populates="usage_logs")

    def __repr__(self) -> str:
        return f"<UsageLog {self.model} {self.total_tokens}tokens>"
