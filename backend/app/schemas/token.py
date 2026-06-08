from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class ApiTokenResponse(BaseModel):
    id: str
    token_prefix: str
    name: str
    is_active: bool
    last_used_at: Optional[datetime] = None
    expires_at: Optional[datetime] = None
    created_at: datetime

    model_config = {"from_attributes": True}


class ApiTokenCreateRequest(BaseModel):
    name: str = ""


class ApiTokenCreatedResponse(BaseModel):
    id: str
    token: str  # Full token shown only on creation
    name: str
    created_at: datetime


class ApiTokenListResponse(BaseModel):
    items: list[ApiTokenResponse]


class ApiTokenRotateResponse(BaseModel):
    id: str
    token: str  # New full token
    name: str
