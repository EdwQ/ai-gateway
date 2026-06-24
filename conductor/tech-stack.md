# Tech Stack

## Backend

| 组件 | 技术 | 用途 |
|---|---|---|
| **语言** | Rust (edition 2021) | 高性能系统编程 |
| **Web 框架** | axum 0.8 | 异步 HTTP 路由与中间件 |
| **运行时** | tokio 1.x (full features) | 异步 I/O 运行时 |
| **HTTP 中间件** | tower 0.4 + tower-http 0.6 (CORS) | 请求管线处理 |
| **数据库 ORM** | sqlx 0.8 (tokio + postgres + uuid + chrono + json + bigdecimal) | 异步 PostgreSQL 访问 |
| **缓存/限流** | redis 0.28 (tokio-comp) | Redis 客户端 |
| **序列化** | serde 1.x + serde_json 1.x | JSON 序列化/反序列化 |
| **JWT** | jsonwebtoken 9 | 认证令牌签发与验证 |
| **加密** | aes-gcm 0.10 (AES-256-GCM) + sha2 0.10 | 数据加密与哈希 |
| **HMAC 签名** | hmac 0.12 | 钉钉 Webhook 签名 |
| **HTTP 客户端** | reqwest 0.12 (json + stream) | 上游 AI Provider 代理 |
| **日志** | tracing 0.1 + tracing-subscriber 0.3 | 结构化日志 |
| **UUID** | uuid 1.x (v4 + serde) | 唯一标识符 |
| **时间处理** | chrono 0.4 + time 0.3 | 日期时间操作 |
| **正则** | regex 1.x | PII 脱敏 |
| **数值** | rust_decimal 1.x (serde) | 金额精确计算 |

## Frontend

| 组件 | 技术 | 用途 |
|---|---|---|
| **语言** | TypeScript | 类型安全的前端开发 |
| **框架** | React | UI 组件化 |
| **构建工具** | Vite | 开发服务器与生产构建 |
| **UI 组件库** | Ant Design | 企业级 UI 组件 |
| **HTTP 客户端** | Axios (通过 @api/client.ts) | API 请求 |

## Database

| 组件 | 版本 | 用途 |
|---|---|---|
| **PostgreSQL** | 16+ | 主数据存储（用户、Token、调用记录、审计日志） |
| **Redis** | 7+ | 限流计数器、缓存、JWT 黑名单 |

## Infrastructure

| 组件 | 技术 | 用途 |
|---|---|---|
| **容器化** | Docker + Docker Compose | 本地开发与生产部署 |
| **CI/CD** | GitHub Actions | 自动构建与镜像推送 |
| **反向代理** | Nginx (生产部署) | 前端静态资源服务 |
| **部署方式** | 单二进制 / Docker / K8s | 灵活部署选项 |
