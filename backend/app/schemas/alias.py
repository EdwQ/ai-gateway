from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class AliasResponse(BaseModel):
    id: str
    alias_name: str
    target_model: str
    description: Optional[str] = None
    is_active: bool
    created_at: datetime

    model_config = {"from_attributes": True}


class AliasListResponse(BaseModel):
    items: list[AliasResponse]


class AliasCreateRequest(BaseModel):
    alias_name: str
    target_model: str
    description: Optional[str] = None
    is_active: bool = True


class AliasUpdateRequest(BaseModel):
    alias_name: Optional[str] = None
    target_model: Optional[str] = None
    description: Optional[str] = None
    is_active: Optional[bool] = None
