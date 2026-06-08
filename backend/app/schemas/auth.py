from datetime import datetime
from decimal import Decimal
from typing import Optional

from pydantic import BaseModel


class DingTalkQRCodeResponse(BaseModel):
    qr_code_url: str


class DingTalkCallbackRequest(BaseModel):
    auth_code: str


class RefreshTokenRequest(BaseModel):
    refresh_token: str


class UserInfo(BaseModel):
    id: str
    name: str
    email: Optional[str] = None
    avatar: Optional[str] = None
    role: str
    department_name: Optional[str] = None
    quota_balance: Decimal
    quota_used: Decimal

    model_config = {"from_attributes": True}


class LoginResponse(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str = "bearer"
    user: UserInfo


class RefreshTokenResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"


class LogoutResponse(BaseModel):
    message: str = "Logged out successfully"


class MeResponse(BaseModel):
    user: UserInfo
