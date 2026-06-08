from fastapi import APIRouter, Depends, HTTPException, Request, status
from fastapi.responses import RedirectResponse
from redis.asyncio import Redis
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.deps import get_current_user
from app.core.config import get_settings
from app.core.database import get_db
from app.core.redis import get_redis
from app.models.user import User
from app.schemas.auth import (
    DingTalkCallbackRequest,
    LoginResponse,
    LogoutResponse,
    MeResponse,
    RefreshTokenRequest,
    RefreshTokenResponse,
    UserInfo,
)
from app.services.auth_service import auth_service

router = APIRouter(prefix="/auth", tags=["Authentication"])


@router.post("/dingtalk/qrcode", response_model=dict)
async def get_dingtalk_qrcode(request: Request):
    """Get DingTalk QR code URL for scanning login.

    Uses FRONTEND_URL from config if available, otherwise falls back to request base URL.
    In dev, set FRONTEND_URL to your LAN IP (e.g., http://192.168.1.13:3000)
    so the phone can reach the callback via WiFi.
    """
    from app.services.dingtalk_service import dingtalk_service

    settings = get_settings()
    if settings.FRONTEND_URL:
        base_url = settings.FRONTEND_URL.rstrip("/")
    else:
        base_url = str(request.base_url).rstrip("/")
    redirect_uri = base_url + "/api/v1/auth/dingtalk/callback"
    url = dingtalk_service.get_qrcode_url(redirect_uri)
    return {"qr_code_url": url}


@router.post("/dev/login", response_model=LoginResponse)
async def dev_login(db: AsyncSession = Depends(get_db), redis: Redis = Depends(get_redis)):
    """[开发模式] 直接创建测试用户并登录."""
    from app.services.auth_service import auth_service
    
    # 创建测试用户
    test_user_id = "test-dev-user-001"
    
    # Check if user exists
    from sqlalchemy import select
    from app.models.user import User
    result = await db.execute(select(User).where(User.user_id == test_user_id))
    user = result.scalar_one_or_none()
    
    import uuid
    from datetime import datetime, timezone
    from decimal import Decimal
    from app.core.config import get_settings
    
    settings = get_settings()
    
    if user is None:
        # Create test user
        user = User(
            id=uuid.uuid4(),
            user_id=test_user_id,
            union_id="dev-" + test_user_id,
            name="Development User",
            email="dev@localhost.local",
            avatar=None,
            department_id="1",
            department_name="研发部",
            title="Engineer",
            role="admin",  # 测试用户有管理员权限
            is_active=True,
            quota_balance=Decimal(str(settings.DEFAULT_QUOTA_AMOUNT)),
            quota_used=Decimal("0"),
            last_login_at=datetime.now(timezone.utc),
        )
        db.add(user)
        await db.flush()
    else:
        if not user.is_active:
            raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="User account is disabled")
        user.last_login_at = datetime.now(timezone.utc)
    
    # Generate JWT tokens
    from app.core.security import create_access_token, create_refresh_token
    access_token = create_access_token(
        data={"sub": str(user.id), "role": user.role}
    )
    refresh_token = create_refresh_token(
        data={"sub": str(user.id)}
    )
    
    return LoginResponse(
        access_token=access_token,
        refresh_token=refresh_token,
        user=UserInfo(
            id=str(user.id),
            name=user.name,
            email=user.email,
            avatar=user.avatar,
            role=user.role,
            department_name=user.department_name,
            quota_balance=user.quota_balance,
            quota_used=user.quota_used,
        ),
    )


@router.get("/dingtalk/callback")
async def dingtalk_callback_get(
    code: str,
    state: str = "login",
    db: AsyncSession = Depends(get_db),
    redis: Redis = Depends(get_redis),
):
    """Handle DingTalk OAuth redirect after QR code scan (GET callback).

    After user scans QR code with DingTalk and confirms on phone,
    DingTalk redirects the desktop browser to this URL with ?code=AUTH_CODE.
    We process the login and redirect the browser back to the frontend with JWT tokens.
    """
    settings = get_settings()
    try:
        result = await auth_service.login_via_dingtalk(code, db, redis)
        access_token = result["access_token"]
        refresh_token = result["refresh_token"]

        # Redirect back to frontend with tokens in URL fragment
        frontend_url = settings.FRONTEND_URL.rstrip("/")
        redirect_url = (
            f"{frontend_url}/login"
            f"?access_token={access_token}"
            f"&refresh_token={refresh_token}"
        )
        return RedirectResponse(url=redirect_url)
    except ValueError as e:
        error_msg = str(e)
        # Provide more user-friendly error messages
        if "不存在的临时授权码" in error_msg or "expired" in error_msg.lower():
            friendly_error = "授权码已过期，请关闭弹窗后重新扫码"
        elif "无效" in error_msg or "invalid" in error_msg.lower():
            friendly_error = "无效的授权码，请重新扫码登录"
        else:
            friendly_error = f"登录失败：{error_msg}"
        
        return RedirectResponse(
            url=f"{settings.FRONTEND_URL.rstrip('/')}/login?error={friendly_error}"
        )


@router.post("/dingtalk/callback", response_model=LoginResponse)
async def dingtalk_callback_post(
    body: DingTalkCallbackRequest,
    db: AsyncSession = Depends(get_db),
    redis: Redis = Depends(get_redis),
):
    """DingTalk登录回调 (POST - for manual auth_code submission)."""
    try:
        result = await auth_service.login_via_dingtalk(body.auth_code, db, redis)
        user: User = result["user"]
        return LoginResponse(
            access_token=result["access_token"],
            refresh_token=result["refresh_token"],
            user=UserInfo(
                id=str(user.id),
                name=user.name,
                email=user.email,
                avatar=user.avatar,
                role=user.role,
                department_name=user.department_name,
                quota_balance=user.quota_balance,
                quota_used=user.quota_used,
            ),
        )
    except ValueError as e:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(e))


@router.post("/refresh", response_model=RefreshTokenResponse)
async def refresh_token(
    body: RefreshTokenRequest,
    redis: Redis = Depends(get_redis),
):
    """Refresh access token using refresh token."""
    try:
        result = await auth_service.refresh_access_token(body.refresh_token, redis)
        return RefreshTokenResponse(access_token=result["access_token"])
    except ValueError as e:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail=str(e))


@router.post("/logout", response_model=LogoutResponse)
async def logout(
    request: Request,
    current_user: User = Depends(get_current_user),
    redis: Redis = Depends(get_redis),
):
    """Logout: blacklist the current JWT."""
    auth_header = request.headers.get("Authorization", "")
    token = auth_header.replace("Bearer ", "")
    await auth_service.logout(token, redis)
    return LogoutResponse()


@router.get("/me", response_model=MeResponse)
async def get_me(current_user: User = Depends(get_current_user)):
    """Get current user info."""
    return MeResponse(
        user=UserInfo(
            id=str(current_user.id),
            name=current_user.name,
            email=current_user.email,
            avatar=current_user.avatar,
            role=current_user.role,
            department_name=current_user.department_name,
            quota_balance=current_user.quota_balance,
            quota_used=current_user.quota_used,
        )
    )
