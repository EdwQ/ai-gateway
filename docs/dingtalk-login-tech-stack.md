# 钉钉扫码登录技术栈 & 排查指南

> 生成时间：2026-06-09

---

## 一、整体架构

```
┌──────────────────────────────────────────────────────────────────────┐
│                     浏览器（3000端口 / 域名）                          │
│                                                                      │
│  1. 打开登录页 → 请求 QR Code URL                                    │
│  2. 弹出新窗口 → 钉钉 OAuth 页面（显示二维码）                         │
│  3. 手机扫码 → 确认授权                                              │
│  4. 钉钉回调 → 302 重定向到前端 /login?access_token=xxx              │
│  5. 前端保存 Token → 跳转到首页                                       │
└──────────────────────────┬──────────────────────────┬────────────────┘
                           │                          │
                           ▼                          ▼
                    ┌──────────────┐         ┌──────────────────┐
                    │  Frontend     │         │  Backend          │
                    │  (3000 / nginx)│        │  (8000 / FastAPI) │
                    │  React + Vite │         │  Python           │
                    └──────────────┘         └────────┬──────────┘
                                                        │
                                                        ▼
                                               ┌──────────────────┐
                                               │  DingTalk API     │
                                               │  (oapi.dingtalk.com)│
                                               └──────────────────┘
```

---

## 二、技术栈清单

### 2.1 前端层 (Frontend)

| 组件 | 技术 | 文件 |
|------|------|------|
| 框架 | React 18 + TypeScript | `frontend/src/` |
| 构建 | Vite 5 | `frontend/vite.config.ts` |
| 路由 | react-router-dom 6 | `Login/index.tsx` |
| UI | Ant Design 5 + @ant-design/icons | `Login/index.tsx` |
| HTTP | Axios (含拦截器) | `api/client.ts` |
| 运行时 | Node 20 (build) → Nginx 静态部署 | `Dockerfile` |
| 部署 | 容器内 Nginx 监听 80，宿主机映射 3000 | `docker-compose.yml` |

**关键前端文件：**
- `frontend/src/pages/Login/index.tsx` — 登录页面 UI + QR Code 弹窗逻辑
- `frontend/src/api/auth.tsx` — AuthProvider 上下文，token 管理
- `frontend/src/api/client.ts` — Axios 实例，API 调用
- `frontend/vite.config.ts` — 开发代理配置 (`/api` → `localhost:8000`)
- `frontend/nginx.conf` — 生产 Nginx 反向代理 (`/api/` → `backend:8000`)

### 2.2 后端层 (Backend)

| 组件 | 技术 | 文件 |
|------|------|------|
| 框架 | FastAPI (Python) | `app/main.py` |
| 运行时 | Python + Uvicorn | `Dockerfile` |
| ORM | SQLAlchemy (async) | `app/models/` |
| DB | PostgreSQL 16 | `docker-compose.yml` |
| Cache | Redis 7 (JWT 黑名单) | `docker-compose.yml` |
| JWT | python-jose + AES-GCM | `app/core/security.py` |
| HTTP | httpx (异步) | `dingtalk_service.py` |
| 配置 | pydantic-settings (.env) | `app/core/config.py` |

**关键后端文件：**
- `backend/app/api/v1/auth.py` — 钉钉登录 API 路由
- `backend/app/services/dingtalk_service.py` — 钉钉 API 客户端
- `backend/app/services/auth_service.py` — 登录逻辑 + 自动注册
- `backend/app/core/config.py` — 配置读取 (DINGTALK_APP_ID 等)
- `backend/app/core/security.py` — JWT 生成/验证
- `backend/app/api/deps.py` — 依赖注入 (get_current_user)

---

## 三、完整登录流程详解

### Step 1: 前端获取 QR Code URL
```
GET /api/v1/auth/dingtalk/qrcode (POST)
→ 后端返回 { qr_code_url: "https://oapi.dingtalk.com/connect/oauth2/sns_authorize?appid=xxx&..." }
```

**后端逻辑** (`auth.py:26-42`):
1. 读取 `FRONTEND_URL` 配置 (或 `request.base_url`)
2. 拼接 `redirect_uri = FRONTEND_URL + /api/v1/auth/dingtalk/callback`
3. 调用 `dingtalk_service.get_qrcode_url(redirect_uri)` 生成钉钉 OAuth URL

### Step 2: 弹出钉钉二维码窗口
```
window.open(qrCodeUrl, 'dingtalk_login', ...)
```

**参数** (`Login/index.tsx:81-98`):
- 窗口尺寸 800×700
- 添加 `t=timestamp` 防止缓存
- QR Code URL 使用 `qrconnect`（方式一：钉钉提供的扫码登录页面）

### Step 3: 手机扫码 → 钉钉回调
```
钉钉 OAuth 服务器 → 302 Redirect
→ GET /api/v1/auth/dingtalk/callback?code=AUTH_CODE&state=login
```

**回调处理** (`auth.py:116-155`):
1. 提取 `code` 参数 (DingTalk auth_code)
2. 调用 `auth_service.login_via_dingtalk(code, db, redis)`
3. 成功 → `302 Redirect` 到 `FRONTEND_URL/login?access_token=xxx&refresh_token=xxx`
4. 失败 → `302 Redirect` 到 `FRONTEND_URL/login?error=xxx`

### Step 4: 后端 DingTalk API 交互（扫码登录第三方网站官方流程）
```
auth_service.login_via_dingtalk(code)
  → dingtalk_service.get_user_info(auth_code)
    1. GET  /gettoken?appkey=xxx&appsecret=xxx         → access_token
    2. POST /sns/getuserinfo_bycode (签名认证)          → {unionid, nick, openid}
       ※ 使用 appKey + appSecret + HMAC-SHA256 签名，不使用 access_token
    3. POST /topapi/user/getbyunionid                  → {userid}
    4. POST /topapi/v2/user/get {userid}               → {email, title, dept}
  → 查询/创建 User 记录
  → 生成 JWT access_token + refresh_token
```

**DingTalk API 端点** (`dingtalk_service.py`):

| 用途 | URL | 说明 |
|------|-----|------|
| 获取 AccessToken | `https://oapi.dingtalk.com/gettoken` | 内部应用凭证 |
| sns临时授权码→用户信息 | `https://oapi.dingtalk.com/sns/getuserinfo_bycode` | 扫码登录专用，需 HMAC-SHA256 签名 |
| unionid→userid | `https://oapi.dingtalk.com/topapi/user/getbyunionid` | 根据 unionid 获取 userid |
| 用户详情 | `https://oapi.dingtalk.com/topapi/v2/user/get` | userid → 详细信息 |
| 部门列表 | `https://oapi.dingtalk.com/topapi/v2/department/listsub` | 组织架构 |
| 部门详情 | `https://oapi.dingtalk.com/topapi/v2/department/get` | 部门名称 |

**注意**：之前错误地使用了 `topapi/v2/user/getuserinfo`（该接口用于企业内部应用免登，接收的是免登码而非 sns临时授权码）。现已修正为正确的 `sns/getuserinfo_bycode` + `getbyunionid` 流程。签名算法为 `HMAC-SHA256(appSecret, timestamp)`，签名数据仅为时间戳（毫秒级字符串）。

### Step 5: 前端处理回调结果
```
URL 参数: ?access_token=xxx&refresh_token=xxx
→ 保存到 localStorage
→ 如果 window.opener 存在 (弹窗模式): 通知父窗口并关闭
→ 否则: 直接跳转到 /
```

**前端处理** (`Login/index.tsx:17-51`):
- 检查 URL 中的 `access_token` / `refresh_token` / `error`
- 弹窗模式下：设置父窗口跳转 → 关闭弹窗
- 非弹窗模式：直接 `window.location.href = '/'`

### Step 6: AuthProvider 初始化
```
localStorage.getItem('access_token') → getMe() → 设置 User 状态
```

**AuthProvider** (`auth.tsx:30-47`):
- 页面加载时读取 localStorage token
- 调用 `/api/v1/auth/me` 获取用户信息
- 失败则清除 token 并跳转登录页

---

## 四、配置项清单

### 必需配置 (.env / docker-compose env)

| 配置项 | 说明 | 示例值 |
|--------|------|--------|
| `DINGTALK_APP_ID` | 钉钉内部应用 AppKey (appid) | `dingXXXX` |
| `DINGTALK_APP_SECRET` | 钉钉内部应用 AppSecret | `***` |
| `DINGTALK_AGENT_ID` | 钉钉应用 AgentId (暂未使用) | `***` |
| `FRONTEND_URL` | 前端可访问地址（回调目标） | `http://localhost:3000` |
| `SECRET_KEY` | JWT 签名密钥 | `min-32-chars` |
| `DATABASE_URL` | PostgreSQL 连接串 | `postgresql+asyncpg://...` |
| `REDIS_URL` | Redis 连接串 | `redis://redis:6379/0` |

### FRONTEND_URL 关键说明
- **开发环境**: `http://localhost:3000`（Nginx 容器映射）
- **生产环境**: `https://dsapi.surebestind.com`（docker-compose.yml 默认值）
- 钉钉扫码后回调到此地址，手机需要能访问到

---

## 五、已知排查重点 & 常见问题

### 5.1 配置问题

#### ❌ DINGTALK_APP_ID / DINGTALK_APP_SECRET 为空
- **表现**: 前端显示 "系统维护中" / QR Code URL 获取失败
- **原因**: `.env` 中钉钉配置为空
- **检查**: 查看 `backend/.env` 和 `docker-compose.yml` 中环境变量是否已设置

#### ❌ FRONTEND_URL 配置不正确
- **表现**: 钉钉扫码后手机端回调失败 / 浏览器收不到重定向
- **原因**: 
  - 容器内 `FRONTEND_URL=localhost:3000` 但手机访问不到容器 localhost
  - 生产环境 `FRONTEND_URL` 与 `ALLOWED_ORIGINS` 不匹配
- **检查**: 
  - `docker-compose.yml` 中 `FRONTEND_URL` 值
  - 开发时建议设为局域网 IP (如 `http://192.168.1.13:3000`)

### 5.2 网络问题

#### ❌ 钉钉回调无法到达前端
- **表现**: 扫码后页面无反应 / 新窗口停留在钉钉页面不跳转
- **原因**: 
  - 钉钉 OAuth 回调 → `/api/v1/auth/dingtalk/callback` → 302 到 FRONTEND_URL
  - 手机可能无法访问 `localhost:3000` 或容器内服务
  - 回调路径 `/api/v1/auth/dingtalk/callback` 通过 Nginx 代理到 `backend:8000`
- **检查**: 
  - 确认 `FRONTEND_URL` 手机端可访问
  - 确认 Nginx `/api/` 代理配置正确
  - 检查后端日志是否有 callback 请求

#### ❌ 弹窗被浏览器拦截
- **表现**: 点击按钮无弹窗 / 控制台有警告
- **原因**: Chrome 等浏览器拦截 `window.open`（需要用户交互触发）
- **检查**: 检查浏览器弹窗拦截提示，允许弹窗

### 5.3 钉钉 API 问题

#### ❌ auth_code 过期
- **表现**: 回调 URL 出现 `?error=授权码已过期`
- **原因**: 钉钉 auth_code 有效期 5 分钟，扫码后需及时处理
- **检查**: 确认扫码→回调流程在 5 分钟内完成

#### ❌ DingTalk API 返回错误
- **表现**: 后端日志出现 `DingTalk getuserinfo error` / `DingTalk token error`
- **原因**: 
  - AppKey/AppSecret 错误
  - 钉钉应用未授权（未添加到企业内部应用）
  - 应用权限不足
- **检查**: 查看后端日志中的错误信息

### 5.4 回调路径问题

#### ❌ 回调 404
- **表现**: 扫码后跳转到错误页面或 404
- **原因**: 
  - Nginx 代理路径 `/api/v1/auth/dingtalk/callback` 匹配了 `/api/` 规则 → proxy_pass `backend:8000`
  - 但如果路径不匹配 `location /api/` 则会直接返回 `index.html` (SPA 路由)
- **检查**: 确认 `/api/` 前缀的 location 匹配 `location /api/`

#### ❌ 回调 405 Method Not Allowed
- **表现**: 回调失败
- **原因**: 
  - `GET /dingtalk/callback` 是 GET 方法
  - `POST /dingtalk/callback` 是 POST 方法（用于手动提交 auth_code）
  - Nginx 或后端配置限制了方法
- **检查**: 确认 FastAPI 路由正确注册了 GET 方法

### 5.5 Token 问题

#### ❌ 保存 token 后页面不跳转
- **表现**: URL 中有 token 参数但页面无反应
- **原因**: React 组件未正确检测 URL 参数
- **检查**: 
  - `useSearchParams` 是否在 Login 组件内正确工作
  - `window.opener` 检测逻辑是否异常

#### ❌ 获取 /auth/me 失败
- **表现**: 保存 token 后仍停留在登录页 / 页面白屏
- **原因**: 
  - token 无效 (JWT 签名或过期)
  - `/api/v1/auth/me` 接口返回 401
  - AuthProvider 清除 token 后未跳转
- **检查**: 浏览器 DevTools → Network → `/auth/me` 响应

---

## 六、各文件职责速查

| 文件 | 职责 | 关键函数/组件 |
|------|------|--------------|
| `Login/index.tsx` | 登录页面 UI | `openDingtalkPopup()`, QR Code 获取, 回调参数处理 |
| `auth.tsx` | Auth 上下文 | `login(authCode)`, `devLogin()`, `initAuth()` |
| `client.ts` | Axios 实例 | `login()`, `getMe()`, 拦截器 |
| `auth.py` | 认证 API 路由 | `GET /dingtalk/callback`, `POST /dingtalk/qrcode`, `POST /dingtalk/callback` |
| `dingtalk_service.py` | 钉钉 API 封装 | `get_user_info()`, `get_qrcode_url()`, `_get_access_token()` |
| `auth_service.py` | 登录业务逻辑 | `login_via_dingtalk()`, 自动注册/登录, JWT 签发 |
| `config.py` | 全局配置 | `DINGTALK_*`, `FRONTEND_URL`, `JWT_*` |
| `security.py` | JWT 工具 | `create_access_token()`, `decode_token()` |
| `deps.py` | 依赖注入 | `get_current_user()` |
| `nginx.conf` | 生产代理 | `/api/` → `backend:8000` |
| `vite.config.ts` | 开发代理 | `/api` → `localhost:8000` |
| `docker-compose.yml` | 容器编排 | 端口映射 3000:80, 环境变量传递 |

---

## 七、调试建议

### 快速定位问题步骤

```
1. 检查配置
   → backend/.env: DINGTALK_APP_ID, DINGTALK_APP_SECRET 是否填写
   → docker-compose.yml: FRONTEND_URL 是否正确 (手机可访问)

2. 检查后端日志
   → docker-compose logs backend
   → 搜索 "dingtalk" / "callback" / "auth_code"

3. 检查浏览器 Network
   → 打开 DevTools → Network → 过滤 /api/v1/auth/
   → 查看 QR Code URL 请求、回调请求

4. 模拟测试
   → curl -X POST http://localhost:3000/api/v1/auth/dingtalk/qrcode
   → 查看返回的 qr_code_url 是否合理

5. 测试回调链路
   → 手动触发回调: curl "http://localhost:3000/api/v1/auth/dingtalk/callback?code=TEST"
   → 查看后端是否正常处理
```

### 关键日志位置
- **后端**: `auth.py` 的 `dingtalk_callback_get()` → 打印 `code` 和 `state`
- **后端**: `dingtalk_service.py` 的 `get_user_info()` → 打印 API 返回
- **前端**: `Login/index.tsx` 的 `fetchQrCode()` → 控制台日志
- **前端**: `Login/index.tsx` 的 URL 参数处理 → 检查 searchParams
