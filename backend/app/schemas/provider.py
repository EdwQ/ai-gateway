from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class ProviderResponse(BaseModel):
    id: str
    name: str
    display_name: str
    base_url: str
    models: list[str]
    is_active: bool
    priority: int
    health_status: str
    rate_limit_qps: int
    created_at: datetime
    updated_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


class ProviderCreateRequest(BaseModel):
    name: str
    display_name: str
    base_url: str
    api_key: str
    models: list[str] = []
    is_active: bool = True
    priority: int = 100
    rate_limit_qps: int = 60


class ProviderUpdateRequest(BaseModel):
    display_name: Optional[str] = None
    base_url: Optional[str] = None
    api_key: Optional[str] = None
    models: Optional[list[str]] = None
    is_active: Optional[bool] = None
    priority: Optional[int] = None
    rate_limit_qps: Optional[int] = None


class ProviderKeyResponse(BaseModel):
    id: str
    is_active: bool
    weight: int
    fail_count: int
    last_success_at: Optional[datetime] = None
    created_at: datetime

    model_config = {"from_attributes": True}


class ProviderListResponse(BaseModel):
    items: list[ProviderResponse]


class HealthCheckResponse(BaseModel):
    status: str
    latency_ms: float
