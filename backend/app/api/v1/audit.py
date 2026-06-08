from typing import Optional

from fastapi import APIRouter, Depends, Query
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user, require_role
from app.core.database import get_db
from app.models.user import User
from app.schemas.audit import AuditLogListResponse, AuditLogResponse
from app.services.audit_service import audit_service

router = APIRouter(prefix="/audit", tags=["Audit"])


@router.get("/logs", response_model=AuditLogListResponse)
async def list_audit_logs(
    page: int = Query(1, ge=1),
    page_size: int = Query(20, ge=1, le=100),
    action: Optional[str] = Query(None),
    user_id: Optional[str] = Query(None),
    resource_type: Optional[str] = Query(None),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["admin", "super_admin"])),
):
    """List audit logs with pagination."""
    result = await audit_service.list_logs(
        db,
        page=page,
        page_size=page_size,
        action=action,
        user_id=user_id,
        resource_type=resource_type,
    )
    return AuditLogListResponse(
        items=[AuditLogResponse.model_validate(log) for log in result["items"]],
        total=result["total"],
        page=result["page"],
        page_size=result["page_size"],
    )
