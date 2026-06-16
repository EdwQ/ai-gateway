# 企业 AI 能力平台（MVP）开发计划 V2.0

> **文档版本**：2.0
> **最后更新**：2026-06-07
> **技术栈**：Python FastAPI + React + PostgreSQL + Redis
> **项目目标**：构建公司内部统一 AI API Gateway，实现钉钉 SSO 登录、统一 API 管理、成本统计与审计。

---

## 一、项目概述

### 1.1 核心目标
建设公司内部统一 AI 能力平台，解决 API Key 重复管理、费用统计困难、使用审计缺失等问题。
- **统一入口**：钉钉扫码登录，自动开户
- **统一管理**：统一 API Key 池，AES256 加密存储
- **统一统计**：按用户、部门、模型统计 Token 及 RMB 消耗
- **统一审计**：Prompt 脱敏保存，敏感操作审计

### 1.2 二期边界（不包含）
- AI Agent 编排
- MCP Server 集成
- 企业知识库 / RAG
- 工作流引擎

---

## 二、技术架构

### 2.1 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                        Client Layer                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐  │
│  │ OpenAI   │  │ Cursor   │  │ Cherry   │  │ Admin       │  │
│  │ SDK      │  │          │  │ Studio   │  │ Web UI      │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬──────┘  │
├───────┴──────────────┴──────────────┴──────────────┴────────┤
│                      Gateway Layer (FastAPI)                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐  │
│  │ Auth     │  │ Rate     │  │ Audit    │  │ OpenAI      │  │
│  │ Middle   │  │ Limiter  │  │ Logging  │  │ Compatible  │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────┬──────┘  │
├────────────────────────────────────────────────────┼────────┤
│                  Provider Router                    │        │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┼──┐     │
│  │OpenAI│Claude│Gemini│DS    │Qwen  │Ollama│vLLM  │...│     │
│  └──────┴──────┴──────┴──────┴──────┴──────┴──────┴───┘     │
├─────────────────────────────────────────────────────────────┤
│                      Data Layer                              │
│  ┌─────────────────────┐  ┌──────────────────────────────┐   │
│  │    PostgreSQL        │  │          Redis               │   │
│  │  - Users             │  │  - JWT Blacklist             │   │
│  │  - API Tokens        │  │  - Rate Limit Counter        │   │
│  │  - Providers/Keys    │  │  - Quota Atomic Deduct       │   │
│  │  - Usage Logs        │  │  - SSE Session Store         │   │
│  │  - Audit Logs        │  │                              │   │
│  └─────────────────────┘  └──────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术栈详情

| 层级 | 技术 | 版本 | 说明 |
|:---|:---|:---|:---|
| 后端框架 | FastAPI | 0.115+ | Python async web 框架 |
| ASGI 服务器 | Uvicorn | 0.32+ | 高性能 ASGI 服务器 |
| ORM | SQLAlchemy | 2.0+ | 异步 ORM |
| 数据库驱动 | asyncpg | 0.30+ | PostgreSQL 异步驱动 |
| 缓存 | Redis | 7.0+ | 通过 redis-py |
| 迁移 | Alembic | 1.13+ | 数据库迁移管理 |
| 前端 | React 18 + TypeScript | - | Vite 构建 |
| UI 库 | Ant Design | 5.x | 企业级 UI |
| 图表 | ECharts | 5.x | BI 可视化 |
| 部署 | Docker Compose | - | 一键部署 |
| 监控 | Prometheus + Grafana | - | 指标监控 |

### 2.3 目录结构

```
ai-gateway/
├── backend/
│   ├── app/
│   │   ├── __init__.py
│   │   ├── main.py                    # FastAPI app entry
│   │   ├── api/
│   │   │   ├── __init__.py
│   │   │   ├── deps.py                # Dependency injection
│   │   │   └── v1/
│   │   │       ├── __init__.py
│   │   │       ├── auth.py            # DingTalk OAuth + JWT
│   │   │       ├── users.py           # User management
│   │   │       ├── tokens.py          # API Token management
│   │   │       ├── gateway.py         # OpenAI compatible proxy
│   │   │       ├── providers.py       # Provider management
│   │   │       ├── admin.py           # Admin dashboard
│   │   │       ├── stats.py           # BI statistics
│   │   │       └── audit.py           # Audit logs
│   │   ├── core/
│   │   │   ├── __init__.py
│   │   │   ├── config.py             # Settings (pydantic-settings)
│   │   │   ├── security.py           # JWT, AES, hash utilities
│   │   │   ├── database.py           # SQLAlchemy async engine
│   │   │   ├── redis.py              # Redis client
│   │   │   └── deps.py               # Shared deps
│   │   ├── models/
│   │   │   ├── __init__.py
│   │   │   ├── base.py               # Declarative base
│   │   │   ├── user.py               # User model
│   │   │   ├── token.py              # ApiToken model
│   │   │   ├── provider.py           # Provider + ProviderKey models
│   │   │   ├── usage.py              # UsageLog model
│   │   │   ├── audit.py              # AuditLog + PromptAudit models
│   │   │   └── department.py         # Department model
│   │   ├── schemas/
│   │   │   ├── __init__.py
│   │   │   ├── auth.py               # Auth request/response schemas
│   │   │   ├── user.py
│   │   │   ├── token.py
│   │   │   ├── provider.py
│   │   │   ├── gateway.py            # OpenAI compatible schemas
│   │   │   ├── usage.py
│   │   │   └── audit.py
│   │   ├── services/
│   │   │   ├── __init__.py
│   │   │   ├── auth_service.py       # DingTalk OAuth logic
│   │   │   ├── user_service.py
│   │   │   ├── token_service.py
│   │   │   ├── gateway_service.py    # Provider routing + proxy
│   │   │   ├── usage_service.py      # Token counting + cost calc
│   │   │   ├── audit_service.py
│   │   │   └── dingtalk_service.py   # DingTalk API client
│   │   ├── middleware/
│   │   │   ├── __init__.py
│   │   │   ├── auth.py               # JWT auth middleware
│   │   │   ├── rate_limit.py         # Rate limiting
│   │   │   ├── audit_log.py          # Request audit logging
│   │   │   └── log_sanitizer.py      # Log sanitization
│   │   └── utils/
│   │       ├── __init__.py
│   │       ├── encrypt.py            # AES encrypt/decrypt
│   │       ├── mask.py               # PII masking
│   │       └── token_counter.py      # Token counting
│   ├── alembic/
│   │   ├── env.py
│   │   └── versions/
│   ├── tests/
│   │   ├── conftest.py
│   │   ├── test_auth.py
│   │   ├── test_gateway.py
│   │   ├── test_tokens.py
│   │   └── test_usage.py
│   ├── requirements.txt
│   ├── Dockerfile
│   └── .env.example
├── frontend/
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── api/                      # API client
│   │   ├── pages/
│   │   │   ├── Login/
│   │   │   ├── Dashboard/
│   │   │   ├── Tokens/
│   │   │   ├── Providers/
│   │   │   ├── Users/
│   │   │   ├── Audit/
│   │   │   └── Stats/
│   │   ├── components/               # Shared components
│   │   ├── hooks/
│   │   ├── store/                    # State management
│   │   └── utils/
│   ├── package.json
│   ├── vite.config.ts
│   └── Dockerfile
├── docker-compose.yml
├── docker-compose.dev.yml
├── .env.example
├── prometheus/
│   └── prometheus.yml
├── grafana/
│   └── dashboards/
└── README.md
```

---

## 三、数据库设计

### 3.1 ER 图（文字描述）

```
User 1──N ApiToken
User 1──N UsageLog
User 1──N AuditLog
UsageLog 1──0..1 PromptAudit
Provider 1──N ProviderKey
User N──1 Department
Department 1──N User
```

### 3.2 表结构

#### users
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | UUID | PK, default uuid4 | 主键 |
| union_id | VARCHAR(128) | UNIQUE, NOT NULL | 钉钉 unionId |
| user_id | VARCHAR(128) | UNIQUE, INDEX | 钉钉 userId |
| name | VARCHAR(128) | NOT NULL | 用户姓名 |
| email | VARCHAR(256) | - | 邮箱 |
| avatar | VARCHAR(512) | - | 头像 URL |
| department_id | VARCHAR(64) | INDEX | 部门 ID |
| department_name | VARCHAR(256) | - | 部门名称 |
| title | VARCHAR(256) | - | 岗位 |
| role | ENUM('employee','admin','finance','super_admin') | DEFAULT 'employee' | 角色 |
| is_active | BOOLEAN | DEFAULT true | 是否启用 |
| quota_balance | DECIMAL(12,4) | DEFAULT 0 | 额度余额(¥) |
| quota_used | DECIMAL(12,4) | DEFAULT 0 | 已使用额度(¥) |
| last_login_at | TIMESTAMP | - | 最后登录时间 |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |
| updated_at | TIMESTAMP | ON UPDATE now() | 更新时间 |

#### api_tokens
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | UUID | PK | 主键 |
| user_id | UUID | FK->users.id, INDEX | 所属用户 |
| token_hash | VARCHAR(256) | UNIQUE, NOT NULL | Token SHA256 哈希 |
| token_prefix | VARCHAR(20) | NOT NULL | Token 前缀(如 sk-company-a1b2) |
| name | VARCHAR(128) | DEFAULT '' | 显示名称 |
| is_active | BOOLEAN | DEFAULT true | 是否有效 |
| last_used_at | TIMESTAMP | - | 最后使用时间 |
| expires_at | TIMESTAMP | - | 过期时间(NULL=永不过期) |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |

#### departments
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | VARCHAR(64) | PK | 钉钉部门 ID |
| name | VARCHAR(256) | NOT NULL | 部门名称 |
| parent_id | VARCHAR(64) | - | 父部门 ID |
| order_num | INT | - | 排序号 |
| is_active | BOOLEAN | DEFAULT true | 是否启用 |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |

#### providers
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | UUID | PK | 主键 |
| name | VARCHAR(64) | UNIQUE, NOT NULL | 名称(openai, deepseek 等) |
| display_name | VARCHAR(128) | NOT NULL | 显示名称 |
| base_url | VARCHAR(512) | NOT NULL | API 基础 URL |
| api_key_encrypted | TEXT | NOT NULL | AES256 加密的 API Key |
| models | JSONB | DEFAULT '[]' | 支持模型列表 |
| is_active | BOOLEAN | DEFAULT true | 是否启用 |
| priority | INT | DEFAULT 100 | 优先级(越小越优先) |
| health_status | ENUM('unknown','healthy','degraded','down') | DEFAULT 'unknown' | 健康状态 |
| rate_limit_qps | INT | DEFAULT 60 | QPS 限制 |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |
| updated_at | TIMESTAMP | ON UPDATE now() | 更新时间 |

#### provider_keys
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | UUID | PK | 主键 |
| provider_id | UUID | FK->providers.id, INDEX | 所属 Provider |
| key_encrypted | TEXT | NOT NULL | AES256 加密 Key |
| is_active | BOOLEAN | DEFAULT true | 是否有效 |
| weight | INT | DEFAULT 1 | 轮询权重 |
| fail_count | INT | DEFAULT 0 | 连续失败次数 |
| max_fail_count | INT | DEFAULT 3 | 最大失败后自动禁用 |
| last_success_at | TIMESTAMP | - | 最后成功时间 |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |

#### usage_logs
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | BIGSERIAL | PK | 主键 |
| user_id | UUID | FK->users.id, INDEX | 用户 ID |
| token_id | UUID | FK->api_tokens.id | 使用的 Token |
| model | VARCHAR(128) | NOT NULL, INDEX | 模型名称 |
| provider | VARCHAR(64) | NOT NULL | Provider 名称 |
| prompt_tokens | INT | DEFAULT 0 | 输入 Token 数 |
| completion_tokens | INT | DEFAULT 0 | 输出 Token 数 |
| total_tokens | INT | DEFAULT 0 | 总 Token 数 |
| cost_rmb | DECIMAL(12,6) | DEFAULT 0 | 费用(元) |
| duration_ms | INT | DEFAULT 0 | 耗时(毫秒) |
| is_stream | BOOLEAN | DEFAULT false | 是否流式 |
| is_success | BOOLEAN | DEFAULT true | 是否成功 |
| status_code | INT | DEFAULT 200 | HTTP 状态码 |
| error_message | TEXT | - | 错误信息 |
| ip_address | INET | - | 请求 IP |
| request_id | VARCHAR(64) | UNIQUE, INDEX | 请求 ID(用于幂等) |
| created_at | TIMESTAMP | DEFAULT now(), INDEX | 创建时间 |

#### prompt_audits
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | BIGSERIAL | PK | 主键 |
| usage_log_id | BIGINT | FK->usage_logs.id, UNIQUE | 关联用量记录 |
| save_mode | ENUM('off','summary','masked','full') | NOT NULL | 保存模式 |
| prompt_content | TEXT | - | Prompt 内容(脱敏后) |
| prompt_summary | VARCHAR(512) | - | Prompt 摘要 |
| completion_content | TEXT | - | 完整输出内容 |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |

#### audit_logs
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | BIGSERIAL | PK | 主键 |
| user_id | UUID | FK->users.id, INDEX | 操作人 |
| action | VARCHAR(64) | NOT NULL, INDEX | 操作类型 |
| resource_type | VARCHAR(64) | NOT NULL | 资源类型 |
| resource_id | VARCHAR(128) | - | 资源 ID |
| details | JSONB | - | 操作详情 |
| ip_address | INET | - | 请求 IP |
| user_agent | VARCHAR(512) | - | User-Agent |
| created_at | TIMESTAMP | DEFAULT now(), INDEX | 创建时间 |

#### jwt_blacklist
| 字段 | 类型 | 约束 | 说明 |
|:---|:---|:---|:---|
| id | UUID | PK | 主键 |
| jti | VARCHAR(64) | UNIQUE, NOT NULL, INDEX | JWT ID |
| expires_at | TIMESTAMP | NOT NULL, INDEX | JWT 过期时间 |
| created_at | TIMESTAMP | DEFAULT now() | 创建时间 |

---

## 四、API 接口设计

### 4.1 认证 (Auth)

| 方法 | 路径 | 说明 | 认证 |
|:---|:---|:---|:---|
| POST | /api/v1/auth/dingtalk/qrcode | 获取钉钉扫码二维码 | No |
| POST | /api/v1/auth/dingtalk/callback | 钉钉扫码回调 (authCode) | No |
| POST | /api/v1/auth/refresh | 刷新 Access Token | Refresh Token |
| POST | /api/v1/auth/logout | 登出 (黑名单) | Access Token |
| GET | /api/v1/auth/me | 获取当前用户信息 | Access Token |

### 4.2 用户管理 (Users)

| 方法 | 路径 | 说明 | 权限 |
|:---|:---|:---|:---|
| GET | /api/v1/users | 用户列表 (分页) | admin+ |
| GET | /api/v1/users/{id} | 用户详情 | admin+ |
| PATCH | /api/v1/users/{id} | 修改用户 (角色/状态/额度) | admin+ |
| DELETE | /api/v1/users/{id} | 禁用用户 | super_admin |

### 4.3 API Token

| 方法 | 路径 | 说明 | 权限 |
|:---|:---|:---|:---|
| GET | /api/v1/tokens | 我的 Token 列表 | Auth |
| POST | /api/v1/tokens | 创建 Token | Auth |
| DELETE | /api/v1/tokens/{id} | 删除/失效 Token | Auth |
| POST | /api/v1/tokens/{id}/rotate | 轮换 Token | Auth |

### 4.4 AI Gateway (OpenAI Compatible)

| 方法 | 路径 | 说明 |
|:---|:---|:---|
| POST | /v1/chat/completions | Chat Completion (OpenAI SDK 兼容) |
| POST | /v1/embeddings | Embedding (OpenAI SDK 兼容) |
| GET | /v1/models | 模型列表 (OpenAI SDK 兼容) |

### 4.5 Provider 管理

| 方法 | 路径 | 说明 | 权限 |
|:---|:---|:---|:---|
| GET | /api/v1/admin/providers | Provider 列表 | admin+ |
| POST | /api/v1/admin/providers | 新增 Provider | admin+ |
| PUT | /api/v1/admin/providers/{id} | 更新 Provider | admin+ |
| DELETE | /api/v1/admin/providers/{id} | 删除 Provider | super_admin |
| POST | /api/v1/admin/providers/{id}/check | 健康检查 | admin+ |

### 4.6 统计 (BI)

| 方法 | 路径 | 说明 | 权限 |
|:---|:---|:---|:---|
| GET | /api/v1/stats/dashboard | Dashboard 概览 | admin+ |
| GET | /api/v1/stats/daily | 每日统计明细 | admin+ |
| GET | /api/v1/stats/monthly | 月度统计 | admin+ |
| GET | /api/v1/stats/export | 导出报表 (CSV/Excel) | finance+ |

### 4.7 审计日志

| 方法 | 路径 | 说明 | 权限 |
|:---|:---|:---|:---|
| GET | /api/v1/audit/logs | 审计日志 (分页) | admin+ |
| GET | /api/v1/audit/prompts | Prompt 审计查询 | admin+ |

### 4.8 监控

| 方法 | 路径 | 说明 |
|:---|:---|:---|
| GET | /health/readiness | Readiness Probe |
| GET | /health/liveness | Liveness Probe |
| GET | /metrics | Prometheus Metrics |

---

## 五、定价模型

### 5.1 模型价格表 (示例)

| 模型 | 输入 ($/M tokens) | 输出 ($/M tokens) | 备注 |
|:---|:---:|:---:|:---|
| gpt-4o | 2.50 | 10.00 | OpenAI |
| gpt-4o-mini | 0.15 | 0.60 | OpenAI |
| claude-3-5-sonnet | 3.00 | 15.00 | Anthropic |
| claude-3-5-haiku | 0.80 | 4.00 | Anthropic |
| gemini-2.0-flash | 0.10 | 0.40 | Google |
| deepseek-chat | 0.14 | 0.28 | DeepSeek |
| deepseek-reasoner | 0.55 | 2.19 | DeepSeek |
| qwen-max | 1.60 | 4.80 | 通义千问 |
| qwen-plus | 0.40 | 1.20 | 通义千问 |

> 汇率按 1 USD = 7.25 RMB 计算，可在系统配置中调整。

---

## 六、安全设计

### 6.1 API Key 加密
- **算法**：AES-256-GCM
- **密钥管理**：主密钥通过环境变量 `ENCRYPTION_KEY` 注入，长度 32 字节
- **存储**：数据库只存密文，后台管理页面展示以 `sk-****xxxx` 呈现
- **轮换**：支持主动 Key 轮换，旧 Key 自动失效

### 6.2 JWT 安全
- **算法**：HS256 (使用随机生成的 SECRET_KEY)
- **Access Token**：30 分钟有效期
- **Refresh Token**：7 天有效期，Redis 中存储 refresh token 映射
- **黑名单**：登出后 jti 加入 Redis 黑名单，TTL 等于 Token 剩余有效期

### 6.3 日志脱敏
统一 Mask 以下字段：
- `Authorization` / `Cookie` 头部
- `api_key` / `apikey` / `token` 参数
- `password` / `secret` 参数
- 手机号: `138****1234`
- 身份证: `110***********1234`
- 邮箱: `u***@example.com`
- 银行卡: `6222****1234`

### 6.4 Prompt 脱敏策略
| 模式 | 行为 | 默认 |
|:---|:---|:---:|
| off | 不保存 Prompt | ✅ |
| summary | 只保存摘要 (前 100 字) | - |
| masked | 保存脱敏后的完整内容 | - |
| full | 保存完整内容 (需超管授权) | - |

---

## 七、原子开发任务清单 (已展开)

### Epic 01：身份认证系统 (Authentication)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-001** | 钉钉扫码登录 | 无 | 接入钉钉 OAuth 2.0，获取 authCode → access_token → unionId → 用户信息 | 1. 员工扫码可登录 2. 非企业用户拒绝 3. 登录耗时 <3s |
| **TASK-002** | 自动开户 | TASK-001 | 首次登录自动创建 User、默认额度、默认 employee 角色、默认 5 个 API Token | 1. DB 新增用户 2. 重复登录不重复创建 |
| **TASK-003** | JWT 签发 | TASK-002 | Access Token(30m) + Refresh Token(7d)；支持 Refresh & Logout Blacklist | 1. JWT 可刷新 2. Logout 立即失效 |

### Epic 02：用户管理 (User Management)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-010** | User CRUD | - | 支持用户禁用/启用、修改额度、修改角色、分页查询 | RBAC 权限校验正常 |
| **TASK-011** | 部门同步 | TASK-001 | 每日同步钉钉部门、岗位、离职状态 | 离职员工自动禁用 |

### Epic 03：API Token 管理 (API Token Mgmt)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-020** | Token 生成 | - | 生成 sk-company-xxxxx，唯一且可复制，存储 SHA256 哈希 | 1. 唯一性 2. 可复制 3. 可失效 |
| **TASK-021** | Token 轮换 | TASK-020 | 重新生成 Token，旧 Token 立即失效 | 旧 Token 调用立即失败 |

### Epic 04：AI Gateway (Core Gateway)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-100** | OpenAI Compatible | - | 兼容 /v1/chat/completions, /v1/embeddings, /v1/models 接口 | OpenAI SDK 可直接调用 |
| **TASK-101** | SSE 流式支持 | TASK-100 | 支持 stream=true 转发，断连自动清理 | Cursor/OpenWebUI 正常接收流 |
| **TASK-102** | 非流式支持 | TASK-100 | 支持 stream=false，标准响应含 usage | usage 统计准确 |
| **TASK-103** | Provider 路由 | - | 支持 OpenAI, Claude, Gemini, DeepSeek, Qwen, Ollama, vLLM | 后台可动态切换 Provider |
| **TASK-104** | 多 Key 轮询 | - | RoundRobin / Weight / Failover（429 自动切换） | 故障 Key 自动剔除，无感知切换 |

### Epic 05：用量统计 (Usage & Billing)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-200** | Token 统计 | - | 统计 Prompt / Completion / Total Tokens | 与 Provider 返回一致 |
| **TASK-201** | RMB 统计 | TASK-200 | 根据模型价格表自动计算费用 | 误差 < 0.1% |
| **TASK-202** | 额度扣减 | - | Redis 原子扣减 + MySQL 最终一致性 | 1000 并发下无超扣 |

### Epic 06：Prompt 审计 (Audit & Compliance)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-300** | 保存策略 | - | 支持：关闭 / 摘要 / 脱敏 / 全文四种模式 | 默认数据库无明文 Prompt |
| **TASK-301** | 敏感信息脱敏 | - | 自动脱敏手机号、身份证、邮箱、银行卡、API Key | DB 中无明文敏感信息 |
| **TASK-302** | 审计日志 | - | 记录用户、时间、模型、耗时、Token、费用、状态 | 支持后台检索与导出 |

### Epic 07：管理后台 (Admin Dashboard)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-400** | Dashboard | - | 统计：用户数、Token 总量、总费用、模型排行 | 首页数据实时准确 |
| **TASK-401** | 用户管理页 | TASK-010 | 搜索、禁用、额度修改、Role 修改 | 权限控制严格 |
| **TASK-402** | Provider 管理 | - | 新增/删除/修改 Provider，健康检查 | 动态生效，无需重启 |
| **TASK-403** | Key 池管理 | - | 展示 Key 健康度、错误率、余额、轮询策略 | 后台可监控 Key 状态 |

### Epic 08：BI 分析 (BI & Reporting)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-500** | 日报统计 | - | 每日 Token、费用、用户、部门占比图表 | 图表展示正常 |
| **TASK-501** | 月报导出 | - | 支持 CSV/Excel 导出月度报表 | 导出文件数据正确 |

### Epic 09：安全体系 (Security)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-600** | API Key 加密 | - | 数据库 AES256-GCM 加密，后台不可见明文 | DB 无明文 Key |
| **TASK-601** | JWT 黑名单 | - | Redis 黑名单机制 | Logout 后 Token 立即失效 |
| **TASK-602** | RBAC 控制 | - | employee(自己) / admin(部门) / finance(统计) / super_admin(全) | 越权返回 403 |
| **TASK-603** | 操作审计 | - | 记录管理员所有敏感操作（查看 Prompt、改额度等） | 日志不可删除 |
| **TASK-604** | Rate Limit | - | 用户 QPS 限制 + Provider 限流 | 恶意刷接口被拦截 |
| **TASK-605** | 日志脱敏 | - | 统一 Mask Authorization / Cookie / Token / Password | 系统日志无敏感泄露 |

### Epic 10：DevOps & 部署 (Infrastructure)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-700** | Docker 部署 | - | Docker Compose 一键启动 (App + DB + Redis + Frontend) | 本地一键 docker compose up |
| **TASK-701** | CI/CD | - | GitHub Action 自动构建、扫描、部署 | Push 触发自动构建 |
| **TASK-702** | Health Check | - | /health/readiness, /health/liveness | K8s 可探测状态 |
| **TASK-703** | 监控指标 | - | Prometheus 暴露 QPS / Latency / SSE / Error / Token | Grafana 可展示仪表盘 |

### Epic 11：测试 (Quality Assurance)
| ID | 任务名 | 依赖 | 开发内容 | 验收标准 (DoD) |
| :--- | :--- | :--- | :--- | :--- |
| **TASK-800** | 单元测试 | - | 核心逻辑覆盖率 > 80% | 单元测试通过 |
| **TASK-801** | 集成测试 | - | Gateway / JWT / Token / Provider 接口测试 | API 测试通过 |
| **TASK-802** | E2E 测试 | - | 登录 → 获取 Key → 调用 AI → 查询日志全流程 | 全流程自动化通过 |
| **TASK-803** | Chaos 测试 | - | 模拟 Redis/MySQL/Provider 故障，SSE 中断 | 系统自动恢复，无死锁 |

---

## 八、开发阶段规划

### Phase 1：身份与基础 (5 天)
- **范围**: Epic 01 (Auth) + Epic 02 (User) + Epic 03 (Token)
- **里程碑**: 钉钉扫码登录 + 自动开户 + API Token 管理
- **后端文件**: `auth.rs`, `users.rs`, `tokens.rs`, `user.rs`, `token.rs` models + services
- **前端文件**: `Login/`, `Tokens/` pages

### Phase 2：核心网关 (5 天)
- **范围**: Epic 04 (Gateway)
- **里程碑**: OpenAI SDK 兼容，支持流式/非流式调用，多 Key 轮询
- **后端文件**: `gateway.rs`, `gateway_service.rs`, `provider.rs` models

### Phase 3：成本中心 (4 天)
- **范围**：Epic 05 (Usage) + Epic 06 (Audit)
- **里程碑**：Token 与 RMB 统计准确，Prompt 脱敏策略生效

### Phase 4：管理后台 (4 天)
- **范围**：Epic 07 (Admin) + Epic 08 (BI)
- **里程碑**：管理员可管理用户、Key 池，查看报表

### Phase 5：上线准备 (3 天)
- **范围**：Epic 09 (Security) + Epic 10 (DevOps) + Epic 11 (Test)
- **里程碑**：安全扫描通过，CI/CD 打通，Chaos 测试通过

---

## 九、环境变量配置

```env
# App
APP_NAME=AI Gateway
DEBUG=false
SECRET_KEY=your-secret-key-32-chars-min
ENCRYPTION_KEY=your-32-byte-aes-key-here!!!!
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173

# Database
DATABASE_URL=postgres://postgres:***@db:5432/ai_gateway

# Redis
REDIS_URL=redis://redis:6379/0

# DingTalk
DINGTALK_APP_ID=your_app_id
DINGTALK_APP_SECRET=your_app_secret
DINGTALK_AGENT_ID=your_agent_id

# JWT
JWT_ACCESS_TOKEN_EXPIRE_MINUTES=30
JWT_REFRESH_TOKEN_EXPIRE_DAYS=7

# Default Quota
DEFAULT_QUOTA_AMOUNT=50.0

# Prompt Audit (off/summary/masked/full)
PROMPT_SAVE_MODE=off

# Rate Limit
RATE_LIMIT_USER_QPS=10
RATE_LIMIT_PROVIDER_QPS=100
```

---

## 十、MVP 验收标准 (Go/No-Go)

| 类别 | 验收项 | 状态 |
| :--- | :--- | :--- |
| **认证** | □ 钉钉扫码登录正常 | □ |
| **账号** | □ 自动开户与离职禁用 | □ |
| **网关** | □ OpenAI SDK 兼容 (Cursor, Cherry Studio) | □ |
| **流式** | □ SSE 流式输出稳定 (断连保护) | □ |
| **统计** | □ Token 统计准确 (误差 <0.1%) | □ |
| **成本** | □ RMB 费用计算准确 | □ |
| **安全** | □ API Key 数据库加密 (AES256) | □ |
| **隐私** | □ Prompt 默认不保存全文 (脱敏策略) | □ |
| **日志** | □ 系统日志无敏感信息泄露 | □ |
| **权限** | □ RBAC 权限隔离正确 (无越权) | □ |
| **运维** | □ 管理后台可正常管理 Key/User | □ |
| **监控** | □ Grafana 监控大盘正常 | □ |
| **部署** | □ Docker 一键部署成功 | □ |
| **测试** | □ 自动化测试 (E2E) 全部通过 | □ |

**满足以上所有条件，方可进入企业内测阶段。**
