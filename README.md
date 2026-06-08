# AI Gateway - 企业 AI 能力平台

> 公司内部统一 AI API Gateway，支持钉钉 SSO 登录、统一 API 管理、成本统计与审计。

---

## 目录

- [快速开始](#快速开始)
- [系统配置](#系统配置)
- [开发指南](#开发指南)
- [API 文档](#api-文档)
- [前端使用指南](#前端使用指南)
- [部署指南](#部署指南)
- [监控与运维](#监控与运维)
- [常见问题](#常见问题)

---

## 快速开始

### 前置条件

- Docker & Docker Compose（推荐）
- 或 Python 3.12+ & Node.js 20+（本地开发）

### 方式一：Docker 一键部署（推荐）

```bash
# 1. 克隆项目
cd /Users/qihe/projects/ai-gateway

# 2. 配置环境变量
cp backend/.env.example backend/.env
# 编辑 .env，填入钉钉应用配置

# 3. 启动所有服务
docker compose up -d

# 4. 访问
# 前端管理后台: http://localhost:3000
# API 文档:      http://localhost:8000/docs
# Prometheus:    http://localhost:9090
# Grafana:       http://localhost:3001 (admin/admin)
```

### 方式二：本地开发模式

**启动依赖服务：**

```bash
docker compose -f docker-compose.dev.yml up -d
# 启动 PostgreSQL (5432) + Redis (6379)
```

**启动后端：**

```bash
cd backend
cp .env.example .env

# 创建虚拟环境（推荐）
python3 -m venv venv
source venv/bin/activate

# 安装依赖
pip install -r requirements.txt

# 初始化数据库（自动建表）
python3 -c "import asyncio; from app.core.database import init_db; asyncio.run(init_db())"

# 启动开发服务器（热重载）
uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
```

**启动前端：**

```bash
cd frontend
npm install
npm run dev
# 访问 http://localhost:5173
```

---

## 系统配置

### 环境变量

| 变量名 | 必填 | 默认值 | 说明 |
|:---|:---:|:---|:---|
| `SECRET_KEY` | ✅ | - | JWT 签名密钥，至少 32 字符 |
| `ENCRYPTION_KEY` | ✅ | - | AES-256 加密密钥，32 字节 |
| `DATABASE_URL` | - | `postgresql+asyncpg://postgres:postgres@db:5432/ai_gateway` | PostgreSQL 连接串 |
| `REDIS_URL` | - | `redis://redis:6379/0` | Redis 连接串 |
| `DINGTALK_APP_ID` | ✅* | - | 钉钉应用 AppKey（*如需登录） |
| `DINGTALK_APP_SECRET` | ✅* | - | 钉钉应用 AppSecret |
| `DINGTALK_AGENT_ID` | - | - | 钉钉应用 AgentId |
| `DEBUG` | - | `false` | 调试模式（开启后自动建表） |
| `ALLOWED_ORIGINS` | - | `http://localhost:3000,http://localhost:5173` | CORS 允许的域名 |
| `PROMPT_SAVE_MODE` | - | `off` | Prompt 保存策略：`off` / `summary` / `masked` / `full` |
| `DEFAULT_QUOTA_AMOUNT` | - | `50.0` | 新用户默认额度（元） |
| `RATE_LIMIT_USER_QPS` | - | `10` | 用户 QPS 上限 |
| `RATE_LIMIT_PROVIDER_QPS` | - | `100` | Provider QPS 上限 |

### 钉钉应用配置

1. 登录 [钉钉开放平台](https://open.dingtalk.com)
2. 创建「企业内部应用」
3. 获取 `AppKey` 和 `AppSecret`
4. 在「应用功能」→「登录与分享」中开启扫码登录
5. 配置回调域名指向本系统

### 模型价格配置

价格表位于 `backend/app/services/gateway_service.py` 中的 `MODEL_PRICES` 字典，格式为：

```python
MODEL_PRICES = {
    "gpt-4o": (2.50, 10.00),      # (输入 $/M tokens, 输出 $/M tokens)
    "deepseek-chat": (0.14, 0.28),
    # ...
}
```

按 1 USD = 7.25 RMB 汇率自动换算，可在代码中调整 `USD_TO_RMB` 常量。

---

## 开发指南

### 项目结构

```
ai-gateway/
├── backend/
│   ├── app/
│   │   ├── api/v1/          # API 路由
│   │   ├── core/            # 核心模块（配置、安全、数据库）
│   │   ├── models/          # SQLAlchemy 数据模型
│   │   ├── schemas/         # Pydantic 请求/响应模型
│   │   ├── services/        # 业务逻辑层
│   │   ├── middleware/      # 中间件（限流、日志脱敏）
│   │   ├── utils/           # 工具函数
│   │   └── main.py          # 应用入口
│   ├── alembic/             # 数据库迁移
│   ├── tests/               # 测试
│   └── Dockerfile
├── frontend/
│   ├── src/
│   │   ├── api/             # API 客户端 & Auth 上下文
│   │   ├── pages/           # 页面组件
│   │   └── App.tsx          # 路由 & 布局
│   └── Dockerfile
├── docker-compose.yml       # 生产部署
├── docker-compose.dev.yml   # 本地开发
└── prometheus/              # 监控配置
```

### 数据库迁移

```bash
cd backend
alembic revision --autogenerate -m "description"
alembic upgrade head
```

### 运行测试

```bash
cd backend
pytest tests/ -v
```

### 添加新 Provider

在管理后台 → Provider 管理中新增即可，无需重启。系统支持：

| Provider | API Base URL |
|:---|:---|
| OpenAI | `https://api.openai.com` |
| Anthropic Claude | `https://api.anthropic.com` |
| Google Gemini | `https://generativelanguage.googleapis.com` |
| DeepSeek | `https://api.deepseek.com` |
| 通义千问 | `https://dashscope.aliyuncs.com` |
| Ollama | `http://localhost:11434` |
| vLLM | `http://localhost:8000` |

---

## API 文档

### 认证 API

#### 获取钉钉扫码二维码

```http
POST /api/v1/auth/dingtalk/qrcode
```

**响应：**
```json
{
  "qr_code_url": "https://oapi.dingtalk.com/connect/qrconnect?appid=..."
}
```

#### 钉钉扫码登录

```http
POST /api/v1/auth/dingtalk/callback
Content-Type: application/json

{
  "auth_code": "dingtalk_auth_code_here"
}
```

**响应：**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "bearer",
  "user": {
    "id": "uuid",
    "name": "张三",
    "email": "zhangsan@company.com",
    "role": "employee",
    "department_name": "技术部",
    "quota_balance": 50.0,
    "quota_used": 12.5
  }
}
```

#### 刷新 Token

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

#### 获取当前用户

```http
GET /api/v1/auth/me
Authorization: Bearer <access_token>
```

### API Token 管理

#### 创建 Token

```http
POST /api/v1/tokens
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "name": "我的 API Key"
}
```

**响应：**
```json
{
  "id": "uuid",
  "token": "sk-company-a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6a7b8c9d0e1f",
  "name": "我的 API Key",
  "created_at": "2026-06-07T12:00:00Z"
}
```

> ⚠️ `token` 字段**仅在此刻返回**，请立即保存！

#### 轮换 Token

```http
POST /api/v1/tokens/{token_id}/rotate
Authorization: Bearer <access_token>
```

### AI Gateway（OpenAI 兼容接口）

#### Chat Completions

```http
POST /v1/chat/completions
Authorization: Bearer sk-company-xxxxx...
Content-Type: application/json

{
  "model": "gpt-4o",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "stream": false
}
```

支持与 OpenAI SDK 完全兼容的使用方式：

```python
from openai import OpenAI

client = OpenAI(
    api_key="sk-company-xxxxx...",
    base_url="http://your-gateway:8000/v1"
)

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

#### 流式响应

将 `stream` 参数设为 `true`，即可获得 SSE（Server-Sent Events）流式响应，支持 Cursor、Cherry Studio、OpenWebUI 等工具。

#### 模型列表

```http
GET /v1/models
Authorization: Bearer sk-company-xxxxx...
```

#### Embeddings

```http
POST /v1/embeddings
Authorization: Bearer sk-company-xxxxx...
Content-Type: application/json

{
  "model": "text-embedding-ada-002",
  "input": "The food was delicious and the waiter..."
}
```

### 管理 API

需 `admin` / `super_admin` / `finance` 角色（取决于接口）。

#### 用户管理

```http
GET /api/v1/users?page=1&page_size=20&search=张三
Authorization: Bearer <admin_token>

PATCH /api/v1/users/{user_id}
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "role": "admin",
  "is_active": true,
  "quota_balance": 100.0
}
```

#### Provider 管理

```http
# 新增 Provider
POST /api/v1/admin/providers
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "name": "deepseek",
  "display_name": "DeepSeek",
  "base_url": "https://api.deepseek.com",
  "api_key": "sk-xxxxxxxx",
  "models": ["deepseek-chat", "deepseek-reasoner"],
  "priority": 100,
  "rate_limit_qps": 60
}

# 健康检查
POST /api/v1/admin/providers/{provider_id}/check
Authorization: Bearer <admin_token>

# 响应
{
  "status": "healthy",
  "latency_ms": 234
}
```

#### 数据统计

```http
# Dashboard 概览
GET /api/v1/stats/dashboard

# 日报
GET /api/v1/stats/daily?days=30

# 月报
GET /api/v1/stats/monthly?months=6

# 导出 CSV
GET /api/v1/stats/export?month=2026-06
```

---

## 前端使用指南

### 登录

1. 访问管理后台
2. 输入钉钉 `auth_code`（MVP 版本）或扫码登录
3. 首次登录自动开户，获得 50 元默认额度

### 获取 API Token

1. 登录后在左侧菜单点击「API Token」
2. 点击「创建 Token」按钮
3. 输入名称（可选），点击确定
4. **立即复制并保存 Token**（关闭弹窗后不再显示）
5. 在 OpenAI SDK 中使用该 Token 调用 AI 接口

### 管理 Provider

> 需要管理员权限

1. 进入「Provider 管理」
2. 点击「新增 Provider」添加 AI 服务商
3. 填写名称、API URL、API Key、支持模型列表
4. 使用「健康检查」按钮测试连通性
5. 支持优先级排序和 QPS 限制

### 查看统计

1. 仪表盘：首页展示关键指标和趋势图
2. 数据统计：日月维度的 Token/费用图表
3. 报表导出：选择月份，一键导出 CSV

### 审计日志

记录所有敏感操作（登录、Token 创建/删除、用户信息修改、Provider 变更等），支持按操作类型筛选。

---

## 部署指南

### 生产部署

```bash
# 1. 配置环境变量
export SECRET_KEY="your-32-char-secret-key"
export ENCRYPTION_KEY="your-32-byte-encryption-key"
export DINGTALK_APP_ID="your-app-id"
export DINGTALK_APP_SECRET="your-app-secret"

# 2. 启动全量服务
docker compose up -d --build

# 3. 验证
curl http://localhost:8000/health/liveness
curl http://localhost:8000/health/readiness
```

### 环境要求

| 组件 | 最低配置 | 推荐配置 |
|:---|:---|:---|
| CPU | 2 核 | 4 核 |
| 内存 | 4 GB | 8 GB |
| 磁盘 | 20 GB | 50 GB |
| Docker | 24+ | 最新版 |

### 性能建议

- PostgreSQL：建议开启 `pg_stat_statements` 用于查询分析
- Redis：建议部署集群模式以支持高可用
- Backend：可通过 `--workers` 参数调整 Worker 数，建议为 CPU 核数 × 2
- Provider Key：建议为每个 Provider 配置多个 Key，系统自动轮询和故障切换

---

## 监控与运维

### 健康检查

```bash
# Liveness Probe（存活检查）
curl http://localhost:8000/health/liveness
# → {"status": "alive", "timestamp": ...}

# Readiness Probe（就绪检查）
curl http://localhost:8000/health/readiness
# → {"status": "ready", "database": "ok", "redis": "ok", ...}
```

### Prometheus 指标

`GET /metrics` 暴露以下指标：

| 指标名 | 类型 | 标签 | 说明 |
|:---|:---:|:---|:---|
| `ai_gateway_requests_total` | Counter | method, endpoint, status | 请求总数 |
| `ai_gateway_request_duration_ms` | Histogram | method, endpoint | 请求延迟 |
| `ai_gateway_sse_streams_total` | Counter | model | SSE 流数 |
| `ai_gateway_tokens_total` | Counter | model, type | Token 用量 |

### 日志脱敏

系统自动对以下敏感信息进行脱敏处理：

| 类型 | 示例 | 脱敏后 |
|:---|:---|:---|
| Authorization 头 | `Bearer sk-company-xxx` | `Bearer ***` |
| 手机号 | `13800138000` | `138****8000` |
| 身份证 | `110101199001011234` | `110101********1234` |
| 邮箱 | `zhangsan@company.com` | `z****n@company.com` |
| 银行卡 | `6222021234561234` | `6222********1234` |

执行 `docker compose logs -f` 可实时查看日志，日志中不包含任何明文敏感信息。

---

## 常见问题

### Q: 启动时提示数据库连接失败？
确保 PostgreSQL 已启动。开发模式请执行：`docker compose -f docker-compose.dev.yml up -d`

### Q: 如何测试 API 兼容性？
```python
# 使用 OpenAI Python SDK 测试
pip install openai
python -c "
from openai import OpenAI
client = OpenAI(api_key='sk-company-xxx', base_url='http://localhost:8000/v1')
models = client.models.list()
print('Models:', [m.id for m in models])
"
```

### Q: 钉钉登录失败？
1. 确认 `.env` 中 `DINGTALK_APP_ID` 和 `DINGTALK_APP_SECRET` 已正确配置
2. 确认钉钉应用已开启扫码登录权限
3. 确认回调域名正确配置

### Q: Provider 健康检查失败？
1. 确认 API Key 正确
2. 确认网络可访问 Provider 的 API
3. 在 Provider 管理页面点击健康检查查看详细错误信息

### Q: 额度消耗不准确？
额度基于模型价格表计算。如需调整，修改 `gateway_service.py` 中的 `MODEL_PRICES` 字典。

### Q: 如何重置所有数据？
```bash
docker compose down -v
docker compose up -d
```

---

## License

企业内部使用。
