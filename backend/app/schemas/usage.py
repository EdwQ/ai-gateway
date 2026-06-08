from decimal import Decimal

from pydantic import BaseModel


class DashboardStats(BaseModel):
    total_users: int
    active_users: int
    total_tokens: int
    total_cost: Decimal
    model_rank: list[dict]


class DailyStats(BaseModel):
    date: str
    total_tokens: int
    total_cost: Decimal
    request_count: int


class MonthlyStats(BaseModel):
    month: str
    total_tokens: int
    total_cost: Decimal
    request_count: int


class DailyStatsResponse(BaseModel):
    items: list[DailyStats]


class MonthlyStatsResponse(BaseModel):
    items: list[MonthlyStats]
