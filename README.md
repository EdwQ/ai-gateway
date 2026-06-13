# AI Gateway

> 企业级 AI API 网关，纯 Rust 实现。统一管理 AI 服务商、API Key、用户额度、调用统计与审计。

**单二进制文件** | **~15 MB 内存占用** | **零外部依赖部署**

---

## 目录

- [系统架构](#系统架构)
- [快速开始](#快速开始)
- [如何使用](#如何使用)
- [API 概览](#api-概览)
- [配置说明](#配置说明)
- [部署指南](#部署指南)
- [服务器需求](#服务器需求)
- [开发指南](#开发指南)
- [常见问题](#常见问题)

---

## 系统架构

```
┌─────────────────────────────────────────────┐
│         客户端 (OpenAI SDK / Cursor)          │
│     base_url = http://gateway:2887/v1         │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│       AI Gateway（纯 Rust，端口 2887）         │
│                                             │
│  AI 代理接口：                                │
│  ├─ /v1/chat/completions  流式/非流式        │
│  ├─ /v1/embeddings        向量嵌入           │
│  └─ /v1/models            模型列表            │
│                                             │
│  管理 API（全部 Rust 实现）：                   │
│  ├─ /api/v1/auth/*      钉钉登录 / JWT       │
│  ├─ /api/v1/users/*     用户 CRUD           │
│  ├─ /api/v1/tokens/*    API Token 管理       │
│  ├─ /api/v1/admin/*     Provider / 别名管理   │
│  ├─ /api/v1/stats/*     统计与报表           │
│  └─ /api/v1/audit/*     审计日志             │
└──────────────┬──────────────────────────────┘
               │
       ┌───────┴───────┐
       ▼               ▼
┌──────────────┐ ┌──────────────┐
│  PostgreSQL   │ │    Redis     │
│  (持久化)     │ │ (限流/缓存)  │
└──────────────┘ └──────────────┘
```

### 部署演变

| 阶段 | 服务数 | 内存 | 语言 |
|------|--------|------|------|
| 原架构 | 5 服务 | ~200 MB | Python + Rust |
| **现架构** | **4 服务** | **~25 MB** | **纯 Rust** |

Python 管理层 ~3500 行代码已全部迁移到 Rust，无需 Python 运行环境。

---

## 快速开始

### 前置条件

- Docker & Docker Compose（推荐部署方式）
- 或 Rust 1.81+ + Node.js 20+（本地开发）
- PostgreSQL 16+ & Redis 7+（本地开发）

### Docker 一键部署

```bash
# 1. 克隆
git clone https://github.com/EdwQ/ai-gateway.git
cd ai-gateway

# 2. 配置环境变量
cp .env.example .env
# 编辑 .env，填入 SECRET_KEY / ENCRYPTION_KEY / 钉钉配置

# 3. 启动
docker compose up -d --build

# 4. 验证
curl http://localhost:2887/health/liveness
# → {"status":"alive","timestamp":...}

# 5. 开发模式下快速登录（获取 JWT）
curl -X POST http://localhost:2887/api/v1/auth/dev/login
# → {"access_token":"eyJ...","user":{"role":"admin",...}}
```

### 本地开发

```bash
# 1. 启动依赖
docker compose -f docker-compose.dev.yml up -d
# PostgreSQL :5432 + Redis :6379

# 2. 初始化数据库表
psql -U postgres -d ai_gateway -f backend-rs/migrations/20240613000001_initial_schema.sql

# 3. 启动 Rust 服务
cd backend-rs
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/ai_gateway \
REDIS_URL=redis://localhost:6379/0 \
cargo run

# 4. 启动前端（可选）
cd frontend
npm install
npm run dev
# → http://localhost:5173
```

---

## 如何使用

### 1. 开发模式登录

系统内置开发模式，无需钉钉配置即可使用：

```bash
# 获取管理员 JWT（自动创建测试用户）
curl -s -X POST http://localhost:2887/api/v1/auth/dev/login \
  -H 'Content-Type: application/json'
```

响应中会返回 `access_token`，后续请求需在 Header 中携带：

```http
Authorization: Bearer eyJ0eXAiOiJKV1QiL...
```

### 2. 创建 API Token

```bash
TOKEN="上面返回的access_token"

curl -X POST http://localhost:2887/api/v1/tokens \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"我的 Key"}'

# 响应（token 字段仅返回一次，请立即保存！）
# {"id":"uuid","token":"sk-company-a1b2c3d4e5f6...","name":"我的 Key"}
```

### 3. 添加 AI Provider

```bash
curl -X POST http://localhost:2887/api/v1/admin/providers \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "deepseek",
    "display_name": "DeepSeek",
    "base_url": "https://api.deepseek.com",
    "api_key": "sk-your-api-key",
    "models": ["deepseek-chat", "deepseek-reasoner"]
  }'
```

### 4. 调用 AI 接口

使用 OpenAI SDK 接入：

```python
from openai import OpenAI

client = OpenAI(
    api_key="sk-company-a1b2c3d4e5f6...",
    base_url="http://localhost:2887/v1"
)

# 列出可用模型
models = client.models.list()
for m in models:
    print(m.id)

# 对话
response = client.chat.completions.create(
    model="deepseek-chat",
    messages=[{"role": "user", "content": "Hello!"}]
)
print(response.choices[0].message.content)
```

Cursor / Cherry Studio / OpenWebUI 等工具配置：

| 配置项 | 值 |
|--------|-----|
| API Base URL | `http://your-gateway:2887/v1` |
| API Key | `sk-company-xxxxx...` |

### 5. 管理后台（前端）

访问 `http://localhost:3000` 打开管理后台：

- **仪表盘**：查看调用量、费用、模型排行
- **API Token**：创建/轮换/删除 Token
- **Provider 管理**：添加/编辑/健康检查 AI 服务商
- **模型别名**：为用户分配可用的模型别名
- **用户管理**：设置角色、额度、可用模型
- **数据统计**：日/月报表 + CSV 导出
- **审计日志**：操作记录追踪

### 6. 模型别名机制

```
用户可见名 → 别名 → 真实模型
  "助手"   →  alias  → deepseek-chat
  "旗舰"   →  alias  → gpt-4o
```

- 管理员创建别名，为用户分配可用的别名列表
- 普通用户只能看到自己被授权的别名
- 切换 Provider 时只需改别名指向，对用户透明

---

## API 概览

### AI 代理接口（端口 2887，需 API Token）

| 端点 | 方法 | 说明 |
|------|------|------|
| `/v1/chat/completions` | POST | 对话（支持流式 SSE） |
| `/v1/embeddings` | POST | 向量嵌入 |
| `/v1/models` | GET | 模型列表（按角色过滤） |

### 管理 API（端口 2887，需 JWT Token）

| 端点 | 方法 | 角色 | 说明 |
|------|------|------|------|
| `/api/v1/auth/dev/login` | POST | - | 开发模式登录 |
| `/api/v1/auth/me` | GET | * | 当前用户信息 |
| `/api/v1/auth/refresh` | POST | - | 刷新 JWT |
| `/api/v1/auth/logout` | POST | - | 登出（黑名单） |
| `/api/v1/tokens` | GET/POST | * | 创建/列出 Token |
| `/api/v1/tokens/{id}/rotate` | POST | * | 轮换 Token |
| `/api/v1/users` | GET | admin+ | 用户分页列表 |
| `/api/v1/users/{id}` | PATCH | admin+ | 编辑用户 |
| `/api/v1/users/{id}` | DELETE | super_admin | 禁用用户 |
| `/api/v1/admin/providers` | GET/POST | admin+ | 管理 Provider |
| `/api/v1/admin/providers/{id}/check` | POST | admin+ | 健康检查 |
| `/api/v1/admin/model-aliases` | GET/POST | admin+ | 管理别名 |
| `/api/v1/stats/dashboard` | GET | * | 仪表盘概览 |
| `/api/v1/stats/daily` | GET | * | 日统计 |
| `/api/v1/stats/export` | GET | finance+ | CSV 导出 |
| `/api/v1/audit/logs` | GET | admin+ | 审计日志 |

---

## 配置说明

### 环境变量

| 变量名 | 必填 | 默认值 | 说明 |
|--------|:----:|--------|------|
| `SECRET_KEY` | ✅ | - | JWT 签名密钥，≥32 字符 |
| `ENCRYPTION_KEY` | ✅ | - | AES-256 加密密钥，32 字节 |
| `DATABASE_URL` | - | `postgresql://postgres:postgres@db:5432/ai_gateway` | PostgreSQL |
| `REDIS_URL` | - | `redis://redis:6379/0` | Redis |
| `DINGTALK_APP_ID` | * | - | 钉钉 AppKey（如需登录） |
| `FRONTEND_URL` | - | `http://localhost:3000` | 前端地址 |
| `RATE_LIMIT_USER_QPS` | - | `10` | 用户 QPS |
| `DEFAULT_QUOTA_AMOUNT` | - | `50.0` | 新用户默认额度（元） |
| `PROMPT_SAVE_MODE` | - | `off` | Prompt 审计模式 |
| `DEBUG` | - | `false` | 调试模式 |

### 模型价格

内置价格表在 `backend-rs/src/proxy.rs` 的 `MODEL_PRICES` 中：

```rust
const MODEL_PRICES: &[(&str, f64, f64)] = &[
    ("gpt-4o",          2.50, 10.00),  // (输入 $/1M tokens, 输出 $/1M tokens)
    ("deepseek-chat",   0.14, 0.28),
    ("claude-3-5-sonnet", 3.00, 15.00),
    // ...
];
```

按 1 USD = 7.25 RMB 自动换算，可在源码中调整。

---

## 部署指南

### Docker 生产部署

```bash
# 单机部署
docker compose up -d --build

# 查看日志
docker compose logs -f rust-proxy

# 更新服务
git pull
docker compose up -d --build rust-proxy
```

### Kubernetes 部署

```yaml
# 最小部署示例
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ai-gateway
spec:
  replicas: 2
  selector:
    matchLabels:
      app: ai-gateway
  template:
    metadata:
      labels:
        app: ai-gateway
    spec:
      containers:
      - name: proxy
        image: ai-gateway:latest
        ports:
        - containerPort: 2887
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ai-gateway-secret
              key: database-url
        - name: SECRET_KEY
          valueFrom:
            secretKeyRef:
              name: ai-gateway-secret
              key: secret-key
```

### 环境要求

| 组件 | 最低配置 | 推荐配置 |
|------|---------|---------|
| CPU | 1 核（x86_64/ARM64） | 2 核 |
| 内存 | 256 MB | 512 MB |
| 磁盘 | 10 GB | 20 GB |
| PostgreSQL | 14+ | 16+ |
| Redis | 6+ | 7+ |

---

## 服务器需求

### 资源估算

| 场景 | 用户数 | 日请求量 | CPU | 内存 | 网络 |
|------|--------|---------|-----|------|------|
| 小团队 | 10-50 人 | 1 万次 | 1 核 | 512 MB | 10 Mbps |
| 中型团队 | 50-200 人 | 10 万次 | 2 核 | 1 GB | 50 Mbps |
| 全公司 | 200-1000 人 | 100 万次 | 4 核 | 2 GB | 100 Mbps |

### 瓶颈分析

| 组件 | 瓶颈 | 扩展方式 |
|------|------|---------|
| **AI Gateway** | CPU（加解密/转发） | 水平扩展（无状态） |
| PostgreSQL | 磁盘 IO（usage_logs 写入） | 读写分离 / 定期归档 |
| Redis | 内存（限流计数器） | 集群模式 |
| 上游 Provider | API 速率限制 | 多 Key 轮询 / 自动故障切换 |

### 推荐服务器配置

```
低成本方案（50 人以内）：
  云服务器 2C4G | ¥50-100/月
  PostgreSQL + Redis + AI Gateway 同机部署
  无需 RDS，自建 PG 完全够用，记得定期 pg_dump 备份

标准方案（200 人以内）：
  云服务器 4C8G | ¥200-400/月
  ⭐ 建议上 RDS PostgreSQL（2C4G 约 ¥2000/年）
```

### 关于 RDS 的建议

AI Gateway 的核心瓶颈在**数据库**（尤其是 `usage_logs` 表的写入），RDS 托管数据库在此场景下优势明显：

| 维度 | 自建 PostgreSQL | RDS |
|------|----------------|-----|
| **运维成本** | 手动备份、修补、调优 | 自动备份、自动故障切换、自动版本修补 |
| **存储扩容** | 停机挂盘，需提前规划 | 一键扩容，零停机，自动增长 |
| **IOPS** | ECS 云盘通常 1000-3000 | 可选高 IOPS 实例 10000+，日志写入不排队 |
| **监控告警** | 自己搭 Prometheus + Grafana | 内置监控 + 微信/钉钉告警 |
| **隐性成本** | 运维时间 = 隐性支出 | 包月付费，成本固定 |

**成本对比：**

```
自建 PG 一年实际成本:
  ECS 额外数据盘 100GB       ¥300
  手动备份到 OSS              ¥100
  你的运维时间 ~10h           ¥3000（隐性）

RDS 2C4G + 50GB SSD 一年:  ≈ ¥2000
  ✅ 含自动备份、一键扩容、免运维
```

**什么时候不需要 RDS：**
- 用户 ≤ 20 人，日请求 ≤ 1000 次 → 自建完全够用
- 数据丢了不心疼 → 不需要备份能力
- 极致省钱 → 自建（务必定时 `pg_dump`）

**一句话：Rust 网关本身才 15MB，瓶颈全在数据库。超过 200 用户或 10 万日请求，上 RDS 省心省力。**

```
高可用方案（1000 人）：
  2 台 4C8G 应用服务器（负载均衡）
  RDS PostgreSQL + Redis 集群
  预计 ¥1000-2000/月
```

---

## 开发指南

### 项目结构

```
ai-gateway/
├── backend-rs/                    # ✅ 唯一后端（纯 Rust）
│   ├── migrations/                #    SQL 迁移文件
│   ├── src/
│   │   ├── main.rs                #    入口：28 条路由注册
│   │   ├── config.rs              #    配置读取
│   │   ├── security.rs            #    JWT / AES / Token
│   │   ├── dingtalk.rs            #    钉钉 OAuth 客户端
│   │   ├── mask.rs                #    PII 脱敏
│   │   ├── proxy.rs               #    AI 代理 + 计费
│   │   ├── auth.rs / rate_limit.rs
│   │   ├── db.rs / redis.rs
│   │   └── routes/
│   │       ├── gateway.rs         #    /v1/* AI 代理
│   │       ├── auth_routes.rs     #    认证
│   │       ├── token_routes.rs    #    Token 管理
│   │       ├── user_routes.rs     #    用户管理
│   │       ├── provider_routes.rs #    Provider 管理
│   │       ├── alias_routes.rs    #    别名管理
│   │       ├── stats_routes.rs    #    统计报表
│   │       ├── audit_routes.rs    #    审计日志
│   │       └── health.rs          #    健康检查
│   └── Cargo.toml
├── frontend/                       # React + Ant Design 管理后台
├── docker-compose.yml              # 生产部署（4 服务）
└── docker-compose.dev.yml          # 本地开发
```

### 常用命令

```bash
# 构建
cd backend-rs && cargo build --release

# 测试
cargo test

# 运行
DATABASE_URL=postgresql://... REDIS_URL=redis://... cargo run

# 数据库迁移（手动）
psql -d ai_gateway -f migrations/20240613000001_initial_schema.sql

# 编译 Release（优化体积）
cargo build --release
# 产物：target/release/ai-gateway-rs  ~15 MB
```

---

## 常见问题

### 启动失败，数据库连接不上？

确保 PostgreSQL 已启动且连接串正确：

```bash
# macOS
brew services start postgresql@16
createdb ai_gateway

# Docker
docker compose -f docker-compose.dev.yml up -d
```

### 如何不用钉钉快速体验？

使用开发模式登录接口，无需任何配置：

```bash
curl -X POST http://localhost:2887/api/v1/auth/dev/login
# 返回 admin 角色的 JWT Token
```

### Provider 健康检查失败？

1. 确认 API Key 和 Base URL 正确
2. 确认服务器可访问外网（某些云厂商需要配置 NAT）
3. 尝试手动 curl 测试：`curl https://api.deepseek.com/v1/models -H "Authorization: Bearer sk-xxx"`

### 如何重置所有数据？

```bash
docker compose down -v
rm -rf postgres_data redis_data
docker compose up -d
```

### 前端页面空白？

确认已设置正确的 API 地址。本地开发时前端默认连接同端口（2887），
如果端口不同，修改 `frontend/src/api/client.ts` 中的 `baseURL`。

---

## License

企业内部使用。
