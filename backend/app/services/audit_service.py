import uuid
from typing import Any, Optional

from sqlalchemy import select, func
from sqlalchemy.ext.asyncio import AsyncSession

from app.models.audit import AuditLog


class AuditService:
    """Audit logging service."""

    async def log(
        self,
        db: AsyncSession,
        user_id: uuid.UUID,
        action: str,
        resource_type: str,
        resource_id: Optional[str] = None,
        details: Optional[dict[str, Any]] = None,
        ip_address: Optional[str] = None,
        user_agent: Optional[str] = None,
    ) -> AuditLog:
        """Create audit log entry."""
        log = AuditLog(
            user_id=user_id,
            action=action,
            resource_type=resource_type,
            resource_id=resource_id,
            details=details,
            ip_address=ip_address,
            user_agent=user_agent,
        )
        db.add(log)
        await db.flush()
        return log

    async def list_logs(
        self,
        db: AsyncSession,
        page: int = 1,
        page_size: int = 20,
        action: Optional[str] = None,
        user_id: Optional[str] = None,
        resource_type: Optional[str] = None,
    ) -> dict:
        """List audit logs with pagination."""
        query = select(AuditLog)

        if action:
            query = query.where(AuditLog.action == action)
        if user_id:
            query = query.where(AuditLog.user_id == user_id)
        if resource_type:
            query = query.where(AuditLog.resource_type == resource_type)

        # Count
        count_query = select(func.count()).select_from(query.subquery())
        total_result = await db.execute(count_query)
        total = total_result.scalar() or 0

        # Paginate
        offset = (page - 1) * page_size
        query = (
            query.order_by(AuditLog.created_at.desc())
            .offset(offset)
            .limit(page_size)
        )
        result = await db.execute(query)
        logs = result.scalars().all()

        return {
            "items": list(logs),
            "total": total,
            "page": page,
            "page_size": page_size,
        }


audit_service = AuditService()
