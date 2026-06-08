from datetime import date, timedelta
from decimal import Decimal
from typing import Optional

from sqlalchemy import select, func, case
from sqlalchemy.ext.asyncio import AsyncSession

from app.models.usage import UsageLog
from app.models.user import User
from app.models.audit import AuditLog


class UsageService:
    """Usage statistics and billing service."""

    async def get_dashboard_stats(self, db: AsyncSession) -> dict:
        """Get dashboard overview statistics."""
        # Total users
        result = await db.execute(select(func.count(User.id)))
        total_users = result.scalar() or 0

        # Active users
        result = await db.execute(
            select(func.count(User.id)).where(User.is_active == True)  # noqa: E712
        )
        active_users = result.scalar() or 0

        # Total tokens and cost
        result = await db.execute(
            select(
                func.coalesce(func.sum(UsageLog.total_tokens), 0),
                func.coalesce(func.sum(UsageLog.cost_rmb), 0),
            ).where(UsageLog.is_success == True)  # noqa: E712
        )
        row = result.one()
        total_tokens = row[0] or 0
        total_cost = Decimal(str(row[1] or 0))

        # Model rank
        result = await db.execute(
            select(
                UsageLog.model,
                func.count(UsageLog.id),
                func.sum(UsageLog.total_tokens),
                func.sum(UsageLog.cost_rmb),
            )
            .where(UsageLog.is_success == True)  # noqa: E712
            .group_by(UsageLog.model)
            .order_by(func.sum(UsageLog.total_tokens).desc())
            .limit(10)
        )
        model_rank = [
            {
                "model": row[0],
                "calls": row[1],
                "total_tokens": row[2] or 0,
                "cost": float(row[3] or 0),
            }
            for row in result.all()
        ]

        return {
            "total_users": total_users,
            "active_users": active_users,
            "total_tokens": total_tokens,
            "total_cost": total_cost,
            "model_rank": model_rank,
        }

    async def get_daily_stats(
        self,
        db: AsyncSession,
        days: int = 30,
        user_id: Optional[str] = None,
        model: Optional[str] = None,
    ) -> list[dict]:
        """Get daily usage statistics."""
        since = date.today() - timedelta(days=days)

        query = select(
            func.date(UsageLog.created_at).label("date"),
            func.coalesce(func.sum(UsageLog.total_tokens), 0).label("tokens"),
            func.coalesce(func.sum(UsageLog.cost_rmb), 0).label("cost"),
            func.count(UsageLog.id).label("requests"),
        ).where(
            UsageLog.created_at >= since,
            UsageLog.is_success == True,  # noqa: E712
        )

        if user_id:
            query = query.where(UsageLog.user_id == user_id)
        if model:
            query = query.where(UsageLog.model == model)

        query = query.group_by(func.date(UsageLog.created_at)).order_by(
            func.date(UsageLog.created_at)
        )
        result = await db.execute(query)
        return [
            {
                "date": str(row[0]),
                "total_tokens": row[1] or 0,
                "total_cost": float(row[2] or 0),
                "request_count": row[3] or 0,
            }
            for row in result.all()
        ]

    async def get_monthly_stats(
        self,
        db: AsyncSession,
        months: int = 6,
    ) -> list[dict]:
        """Get monthly usage statistics."""
        since = date.today() - timedelta(days=months * 30)

        query = select(
            func.date_trunc("month", UsageLog.created_at).label("month"),
            func.coalesce(func.sum(UsageLog.total_tokens), 0).label("tokens"),
            func.coalesce(func.sum(UsageLog.cost_rmb), 0).label("cost"),
            func.count(UsageLog.id).label("requests"),
        ).where(
            UsageLog.created_at >= since,
            UsageLog.is_success == True,  # noqa: E712
        )

        query = query.group_by(func.date_trunc("month", UsageLog.created_at)).order_by(
            func.date_trunc("month", UsageLog.created_at)
        )
        result = await db.execute(query)
        return [
            {
                "month": str(row[0])[:7],
                "total_tokens": row[1] or 0,
                "total_cost": float(row[2] or 0),
                "request_count": row[3] or 0,
            }
            for row in result.all()
        ]


usage_service = UsageService()
