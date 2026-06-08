import hashlib
import hmac
import time as time_module
from typing import Any, Optional, cast

import httpx

from app.core.config import get_settings

settings = get_settings()


class DingTalkService:
    """DingTalk OAuth & API client for internal enterprise apps.

    Correct flow for QR code login (扫码登录第三方网站):
      1. GET  /gettoken?appkey=xxx&appsecret=xxx           → access_token
      2. POST /sns/getuserinfo_bycode?accessKey=xxx&timestamp=xxx&signature=xxx
         Body: {"tmp_auth_code": "CODE"}                    → {unionid, nick, openid}
         NOTE: Uses appKey + appSecret + HMAC signature, NOT access_token.
      3. POST /topapi/user/getbyunionid?access_token=TOKEN
         Body: {"unionid": "UNIONID"}                      → {userid}
      4. POST /topapi/v2/user/get?access_token=TOKEN
         Body: {"userid": "USERID"}                        → full user details
    """

    # Internal app access token
    TOKEN_URL = "https://oapi.dingtalk.com/gettoken"
    # SNs temporary code → user info (QR code login)
    SNS_GET_USER_INFO_URL = "https://oapi.dingtalk.com/sns/getuserinfo_bycode"
    # UnionID → UserID
    GET_BY_UNIONID_URL = "https://oapi.dingtalk.com/topapi/user/getbyunionid"
    # Full user details by userid
    USER_DETAIL_URL = "https://oapi.dingtalk.com/topapi/v2/user/get"
    # DingTalk QR code login URL (方式一: 使用钉钉提供的扫码登录页面)
    QR_CONNECT_URL = "https://oapi.dingtalk.com/connect/qrconnect"
    # Department APIs
    DEPARTMENT_LIST_URL = "https://oapi.dingtalk.com/topapi/v2/department/listsub"
    DEPARTMENT_GET_URL = "https://oapi.dingtalk.com/topapi/v2/department/get"

    def __init__(self):
        self._access_token: Optional[str] = None
        self._access_token_expires: float = 0

    async def _get_access_token(self) -> str:
        """Get internal app access token with caching."""
        if self._access_token and time_module.time() < self._access_token_expires:
            return cast(str, self._access_token)

        async with httpx.AsyncClient() as client:
            resp = await client.get(
                self.TOKEN_URL,
                params={
                    "appkey": settings.DINGTALK_APP_ID,
                    "appsecret": settings.DINGTALK_APP_SECRET,
                },
            )
            data = resp.json()
            if data.get("errcode") != 0:
                raise ValueError(f"DingTalk token error: {data.get('errmsg')}")

            self._access_token = data["access_token"]
            # Expire 10 minutes early to be safe
            self._access_token_expires = time_module.time() + data.get("expires_in", 7200) - 600
            return cast(str, self._access_token)

    def _compute_signature(self, app_secret: str, timestamp: str) -> str:
        """Compute HMAC-SHA256 signature for sns/getuserinfo_bycode.

        Signature formula (per DingTalk docs):
          signature = HMAC-SHA256(appSecret, timestamp)
          where timestamp is the millisecond timestamp as string.
        """
        # HMAC-SHA256 with key=appSecret, message=timestamp
        key_bytes = app_secret.encode("utf-8")
        msg_bytes = timestamp.encode("utf-8")
        digest = hmac.new(key_bytes, msg_bytes, digestmod=hashlib.sha256).digest()
        # Base64 encode
        import base64
        return base64.b64encode(digest).decode("utf-8")

    async def get_user_info(self, auth_code: str) -> dict[str, Any]:
        """Get user info via auth_code from DingTalk QR code login.

        Correct flow (QR code login for 3rd-party websites):
           1. GET /gettoken?appkey=xxx&appsecret=xxx → access_token
           2. POST /sns/getuserinfo_bycode (with signature) → {unionid, nick}
           3. POST /topapi/user/getbyunionid → {userid}
           4. POST /topapi/v2/user/get → full details
        """
        token = await self._get_access_token()

        # Step 1: exchange sns temporary auth_code for unionid via sns/getuserinfo_bycode
        # NOTE: This API uses appKey + appSecret + signature, NOT access_token.
        timestamp = str(int(time_module.time() * 1000))
        signature = self._compute_signature(settings.DINGTALK_APP_SECRET, timestamp)

        async with httpx.AsyncClient() as client:
            resp = await client.post(
                self.SNS_GET_USER_INFO_URL,
                params={
                    "accessKey": settings.DINGTALK_APP_ID,
                    "timestamp": timestamp,
                    "signature": signature,
                },
                json={"tmp_auth_code": auth_code},
            )
            data = resp.json()
            if data.get("errcode") != 0:
                raise ValueError(
                    f"DingTalk sns/getuserinfo_bycode error: {data.get('errmsg', 'unknown')}"
                )

            user_info_node = data.get("user_info", {})
            union_id = user_info_node.get("unionid")
            if not union_id:
                raise ValueError("Failed to get unionid from DingTalk sns API")

            user_info: dict[str, Any] = {
                "unionid": union_id,
                "nick": user_info_node.get("nick", ""),
                "openid": user_info_node.get("openid"),
            }

        # Step 2: unionid → userid via topapi/user/getbyunionid
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                self.GET_BY_UNIONID_URL,
                params={"access_token": token},
                json={"unionid": union_id},
            )
            unionid_data = resp.json()
            if unionid_data.get("errcode") != 0:
                raise ValueError(
                    f"DingTalk getbyunionid error: {unionid_data.get('errmsg', 'unknown')}"
                )

            # The result field contains userid
            unionid_result = unionid_data.get("result", {})
            userid = unionid_result.get("userid")
            if not userid:
                raise ValueError("Failed to get userid from DingTalk getbyunionid")

            user_info["userid"] = userid
            # Some versions also return name/avatar in this response
            user_info["name"] = unionid_result.get("name", user_info.get("nick", ""))
            user_info["avatar"] = unionid_result.get("avatar")

        # Step 3: get full user details by userid
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                self.USER_DETAIL_URL,
                params={"access_token": token},
                json={"userid": userid, "language": "zh_CN"},
            )
            detail_data = resp.json()
            if detail_data.get("errcode") == 0:
                detail_result = detail_data.get("result", {})
                user_info.update({
                    "name": detail_result.get("name", user_info.get("name", "")),
                    "email": detail_result.get("email"),
                    "avatar": detail_result.get("avatar", user_info.get("avatar")),
                    "title": detail_result.get("title"),
                    "dept_id_list": detail_result.get("dept_id_list"),
                    "mobile": detail_result.get("mobile"),
                    "unionid": detail_result.get("unionid", union_id),
                })

        return user_info

    async def get_departments(self, dept_id: str = "1") -> list[dict]:
        """Get sub-departments list."""
        token = await self._get_access_token()
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                self.DEPARTMENT_LIST_URL,
                params={"access_token": token},
                json={"dept_id": dept_id},
            )
            data = resp.json()
            if data.get("errcode") != 0:
                raise ValueError(f"DingTalk dept list error: {data.get('errmsg')}")
            return data.get("result", [])

    async def get_department_detail(self, dept_id: str) -> dict:
        """Get department details."""
        token = await self._get_access_token()
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                self.DEPARTMENT_GET_URL,
                params={"access_token": token},
                json={"dept_id": dept_id},
            )
            data = resp.json()
            if data.get("errcode") != 0:
                raise ValueError(f"DingTalk dept detail error: {data.get('errmsg')}")
            return data.get("result", {})

    def get_qrcode_url(self, redirect_uri: str, state: str = "login") -> str:
        """Generate DingTalk QR code login URL.

        Uses 方式一 (standalone QR code page provided by DingTalk):
        - https://oapi.dingtalk.com/connect/qrconnect
        - Opens DingTalk's own QR code page in a new browser window
        - After user scans QR code and confirms, DingTalk redirects to our callback URL
        - URL encoded redirect_uri for safety
        
        Note: Uses qrconnect (not sns_authorize) because this is for popup window login,
        not embedded QR code (方式二).
        """
        import urllib.parse
        # URL encode redirect_uri properly
        encoded_redirect = urllib.parse.quote(redirect_uri, safe='')
        
        # 方式一: Use DingTalk's standalone QR code page
        return (
            f"{self.QR_CONNECT_URL}"
            f"?appid={settings.DINGTALK_APP_ID}"
            f"&response_type=code"
            f"&scope=snsapi_login"
            f"&state={state}"
            f"&redirect_uri={encoded_redirect}"
        )


dingtalk_service = DingTalkService()
