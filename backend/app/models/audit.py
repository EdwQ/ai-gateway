import uuid
from datetime import datetime
from typing import Optional

from sqlalchemy import (
    DateTime,
    ForeignKey,
    Integer,
    String,
    Text,
    func,
)
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class AuditLog(Base):
    __tablename__ = "audit_logs"

    id: Mapped[int] = mapped_column(
        Integer, primary_key=True, autoincrement=True
    )
    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id"), nullable=False, index=True
    )
    action: Mapped[str] = mapped_column(
        String(64), nullable=False, index=True
    )
    resource_type: Mapped[str] = mapped_column(String(64), nullable=False)
    resource_id: Mapped[Optional[str]] = mapped_column(
        String(128), nullable=True
    )
    details: Mapped[Optional[dict]] = mapped_column(JSONB, nullable=True)
    ip_address: Mapped[Optional[str]] = mapped_column(
        String(45), nullable=True
    )
    user_agent: Mapped[Optional[str]] = mapped_column(
        String(512), nullable=True
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), index=True
    )

    def __repr__(self) -> str:
        return f"<AuditLog {self.action} by {self.user_id}>"


class PromptAudit(Base):
    __tablename__ = "prompt_audits"

    id: Mapped[int] = mapped_column(
        Integer, primary_key=True, autoincrement=True
    )
    usage_log_id: Mapped[int] = mapped_column(
        Integer, ForeignKey("usage_logs.id"), unique=True, nullable=False
    )
    save_mode: Mapped[str] = mapped_column(
        String(32), nullable=False
    )  # off, summary, masked, full
    prompt_content: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    prompt_summary: Mapped[Optional[str]] = mapped_column(
        String(512), nullable=True
    )
    completion_content: Mapped[Optional[str]] = mapped_column(
        Text, nullable=True
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )

    def __repr__(self) -> str:
        return f"<PromptAudit mode={self.save_mode}>"
