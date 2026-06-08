from datetime import datetime

from sqlalchemy import DateTime, func
from sqlalchemy.orm import Mapped, mapped_column, declared_attr

from app.core.database import Base


class TimestampMixin:
    """Mixin adding created_at and updated_at timestamps."""

    @declared_attr
    def created_at(cls) -> Mapped[datetime]:
        return mapped_column(
            DateTime(timezone=True), server_default=func.now(), nullable=False
        )

    @declared_attr
    def updated_at(cls) -> Mapped[datetime | None]:
        return mapped_column(
            DateTime(timezone=True), onupdate=func.now(), nullable=True
        )


__all__ = ["Base", "TimestampMixin"]
