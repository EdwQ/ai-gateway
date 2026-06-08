from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class AuditLogResponse(BaseModel):
    id: int
    user_id: str
    action: str
    resource_type: str
    resource_id: Optional[str] = None
    details: Optional[dict] = None
    ip_address: Optional[str] = None
    user_agent: Optional[str] = None
    created_at: datetime

    model_config = {"from_attributes": True}


class AuditLogListResponse(BaseModel):
    items: list[AuditLogResponse]
    total: int
    page: int
    page_size: int


class PromptAuditResponse(BaseModel):
    id: int
    usage_log_id: int
    save_mode: str
    prompt_content: Optional[str] = None
    prompt_summary: Optional[str] = None
    completion_content: Optional[str] = None
    created_at: datetime

    model_config = {"from_attributes": True}
