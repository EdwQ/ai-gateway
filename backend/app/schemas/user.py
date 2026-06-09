from datetime import datetime
from decimal import Decimal
from typing import Optional

from pydantic import BaseModel


class UserResponse(BaseModel):
    id: str
    union_id: str
    name: str
    email: Optional[str] = None
    avatar: Optional[str] = None
    department_id: Optional[str] = None
    department_name: Optional[str] = None
    title: Optional[str] = None
    role: str
    allowed_models: list[str] = []
    is_active: bool
    quota_balance: Decimal
    quota_used: Decimal
    last_login_at: Optional[datetime] = None
    created_at: datetime
    updated_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


class UserListResponse(BaseModel):
    items: list[UserResponse]
    total: int
    page: int
    page_size: int


class UserUpdateRequest(BaseModel):
    name: Optional[str] = None
    email: Optional[str] = None
    role: Optional[str] = None
    is_active: Optional[bool] = None
    quota_balance: Optional[Decimal] = None
    allowed_models: Optional[list[str]] = None
