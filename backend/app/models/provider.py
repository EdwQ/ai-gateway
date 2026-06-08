import uuid
from datetime import datetime
from typing import Optional

from sqlalchemy import Boolean, DateTime, Integer, String, Text, func, ForeignKey
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.models.base import Base, TimestampMixin


class Provider(Base, TimestampMixin):
    __tablename__ = "providers"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    name: Mapped[str] = mapped_column(
        String(64), unique=True, nullable=False
    )
    display_name: Mapped[str] = mapped_column(String(128), nullable=False)
    base_url: Mapped[str] = mapped_column(String(512), nullable=False)
    api_key_encrypted: Mapped[str] = mapped_column(Text, nullable=False)
    models: Mapped[list] = mapped_column(JSONB, default=list)
    is_active: Mapped[bool] = mapped_column(Boolean, default=True)
    priority: Mapped[int] = mapped_column(Integer, default=100)
    health_status: Mapped[str] = mapped_column(
        String(32), default="unknown"
    )  # unknown, healthy, degraded, down
    rate_limit_qps: Mapped[int] = mapped_column(Integer, default=60)

    keys: Mapped[list["ProviderKey"]] = relationship(
        "ProviderKey", back_populates="provider", cascade="all, delete-orphan"
    )

    def __repr__(self) -> str:
        return f"<Provider {self.name}>"


class ProviderKey(Base):
    __tablename__ = "provider_keys"

    id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), primary_key=True, default=uuid.uuid4
    )
    provider_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), 
        ForeignKey("providers.id"),  # Add foreign key reference
        index=True, 
        nullable=False
    )
    key_encrypted: Mapped[str] = mapped_column(Text, nullable=False)
    is_active: Mapped[bool] = mapped_column(Boolean, default=True)
    weight: Mapped[int] = mapped_column(Integer, default=1)
    fail_count: Mapped[int] = mapped_column(Integer, default=0)
    max_fail_count: Mapped[int] = mapped_column(Integer, default=3)
    last_success_at: Mapped[Optional[datetime]] = mapped_column(
        DateTime(timezone=True), nullable=True
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )

    provider: Mapped["Provider"] = relationship(
        "Provider", back_populates="keys"
    )

    def __repr__(self) -> str:
        return f"<ProviderKey {self.id}>"
