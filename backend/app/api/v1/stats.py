from typing import Optional

from fastapi import APIRouter, Depends, Query
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user
from app.core.database import get_db
from app.models.user import User
from app.schemas.usage import (
    DashboardStats,
    DailyStatsResponse,
    MonthlyStatsResponse,
)
from app.services.usage_service import usage_service

router = APIRouter(prefix="/stats", tags=["Statistics & BI"])


@router.get("/dashboard", response_model=DashboardStats)
async def get_dashboard(
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get dashboard overview statistics."""
    stats = await usage_service.get_dashboard_stats(db)
    return DashboardStats(**stats)


@router.get("/daily", response_model=DailyStatsResponse)
async def get_daily_stats(
    days: int = Query(30, ge=1, le=365),
    user_id: Optional[str] = Query(None),
    model: Optional[str] = Query(None),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get daily usage statistics."""
    items = await usage_service.get_daily_stats(
        db, days=days, user_id=user_id, model=model
    )
    return DailyStatsResponse(items=items)


@router.get("/monthly", response_model=MonthlyStatsResponse)
async def get_monthly_stats(
    months: int = Query(6, ge=1, le=24),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get monthly usage statistics."""
    items = await usage_service.get_monthly_stats(db, months=months)
    return MonthlyStatsResponse(items=items)


@router.get("/export")
async def export_stats(
    month: str = Query(..., description="Month in YYYY-MM format"),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role(["finance", "super_admin"])),
):
    """Export monthly usage report as CSV."""
    import csv
    import io

    from fastapi.responses import StreamingResponse
    from sqlalchemy import select

    from app.models.usage import UsageLog

    result = await db.execute(
        select(UsageLog).where(
            UsageLog.created_at >= f"{month}-01",
            UsageLog.created_at < f"{month}-01T00:00:00+08:00",
        )
    )
    logs = result.scalars().all()

    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerow([
        "Date", "User ID", "Model", "Provider", "Prompt Tokens",
        "Completion Tokens", "Total Tokens", "Cost (RMB)",
        "Duration (ms)", "Success", "Stream",
    ])
    for log in logs:
        writer.writerow([
            log.created_at.isoformat() if log.created_at else "",
            str(log.user_id), log.model, log.provider,
            log.prompt_tokens, log.completion_tokens, log.total_tokens,
            float(log.cost_rmb), log.duration_ms,
            log.is_success, log.is_stream,
        ])

    output.seek(0)
    return StreamingResponse(
        iter([output.getvalue()]),
        media_type="text/csv",
        headers={
            "Content-Disposition": f"attachment; filename=usage_{month}.csv"
        },
    )
