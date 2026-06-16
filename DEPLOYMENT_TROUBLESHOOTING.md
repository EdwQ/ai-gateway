# AI Gateway 部署问题总结与改进建议

> **日期**: 2026-06-16  
> **技术栈**: Rust (Axum), React, PostgreSQL, Redis, Docker  
> **状态**: 生产环境首次部署复盘

---

## 一、遇到的问题及解决方案

### 1. 钉钉登录配置错误

#### 问题描述
- **错误 1**: `无效的 appid: dev_app_id`  
  **原因**：`.env` 文件中使用了占位值 `dev_app_id` 未替换。
- **错误 2**: `无效的 appid: 390cf4fa-3e6c-4aff-9b2e-7c6a32ee4aeb`  
  **原因**：误将飞书 (Feishu) 格式的 UUID 当作钉钉 (DingTalk) App ID 使用。
  **解决**：钉钉 App ID 格式通常为 `cnxxx` 或 `dinggqxxx`，需从钉钉开发者后台获取正确的 Key。
- **错误 3**: `url 参数和应用配置的回调域名不匹配`  
  **原因**：钉钉后台未配置正确的回调域名，或 `.env` 中的 `FRONTEND_URL` 与实际域名不一致。

#### 解决方案
1. **钉钉后台配置**：
   - 回调域名：`apidashboard.surebestind.com`
   - 授权回调地址：`https://apidashboard.surebestind.com/api/v1/auth/dingtalk/callback`
2. **环境变量更新**：
   - `FRONTEND_URL=https://apidashboard.surebestind.com`
   - `ALLOWED_ORIGINS=https://apidashboard.surebestind.com`
3. **凭证校验**：增加启动时校验，若 App ID 格式不符合钉钉规范（非 `cn`/`dinggq` 开头）则报错退出。

#### 根本原因
- 缺乏环境区分机制（开发/生产）。
- 钉钉与飞书凭证格式混淆，无格式校验。
- 回调域名配置分散（`.env`、钉钉后台、前端构建配置）。

---

### 2. Docker 容器环境变量未更新

#### 问题描述
修改 `.env` 后，仅执行 `docker-compose restart` **不会**重新加载环境变量，容器仍使用旧值。

#### 解决方案
**正确做法**：
```bash
docker-compose stop && docker-compose up -d
# 或
docker-compose up -d --force-recreate
```

#### 根本原因
Docker 容器在**启动时**（创建阶段）读取 `.env`，重启（restart）不会重新读取。

#### 改进建议
- 编写 `deploy.sh` 脚本，自动执行 `stop` + `up -d --force-recreate`。
- 在 CI/CD 流程中强制重建容器。

---

### 3. 前端环境变量未更新

#### 问题描述
修改 `.env` 后，前端容器仍使用构建时写入的 `VITE_API_BASE_URL=http://localhost:3000`。

#### 根本原因
Vite 在 **构建时 (Build Time)** 将环境变量编译进代码，运行时无法动态读取。

#### 解决方案
- **方案 A (推荐)**：使用 Nginx 作为入口，通过模板替换 `index.html` 中的配置。
- **方案 B**：通过 Docker entrypoint 脚本生成 `config.json` 供前端动态加载。
- **方案 C**：分离配置为外部 JSON/YAML 文件，前端运行时请求该配置。

---

### 4. 数据库表未初始化

#### 问题描述
首次部署时，数据库为空，缺少 `users` 等表，导致 `relation "users" does not exist` 错误。

#### 解决方案
**临时解决**：手动执行迁移
```bash
docker-compose exec backend sqlx migrate run
# 或
docker-compose exec backend cargo sqlx migrate run
```

#### 根本原因
项目缺乏自动迁移机制，未配置 Docker 启动时自动执行 `sqlx migrate run`。

#### 改进建议
- 在 `docker-compose.yml` 中添加 `init` 容器或修改 `entrypoint` 脚本，在启动服务前自动执行迁移。
- 或集成 `sqlx migrate run` 到 Rust 启动流程（`main.rs` 启动前自动检查并迁移）。

---

## 二、关键配置清单

### 1. `.env` 必需配置项（生产环境）

```bash
# App
APP_NAME=AI Gateway
DEBUG=false
SECRET_KEY=your-secure-random-32-char-key
ENCRYPTION_KEY=your-32-byte-aes-key
FRONTEND_URL=https://apidashboard.surebestind.com
ALLOWED_ORIGINS=https://apidashboard.surebestind.com

# Database
DATABASE_URL=postgres://user:password@db:5432/ai_gateway

# Redis
REDIS_URL=redis://redis:6379/0

# DingTalk
DINGTALK_APP_ID=cnxxx_or_dinggqxxx  # 必须是钉钉格式，非 UUID
DINGTALK_APP_SECRET=your_real_secret
DINGTALK_AGENT_ID=your_agent_id

# JWT
JWT_ACCESS_TOKEN_EXPIRE_MINUTES=30
JWT_REFRESH_TOKEN_EXPIRE_DAYS=7

# Default Quota
DEFAULT_QUOTA_AMOUNT=50.0

# Prompt Audit (off/summary/masked/full)
PROMPT_SAVE_MODE=masked

# Rate Limit
RATE_LIMIT_USER_QPS=10
RATE_LIMIT_PROVIDER_QPS=100
```

### 2. 钉钉后台配置

| 配置项 | 值 |
| :--- | :--- |
| **回调域名** | `apidashboard.surebestind.com` |
| **授权回调地址** | `https://apidashboard.surebestind.com/api/v1/auth/dingtalk/callback` |
| **应用状态** | **已发布** (未发布则无法在生产环境使用) |

---

## 三、标准部署流程

### 1. 准备环境
- 确认服务器可访问 `https://<你的域名>`。
- 配置 SSL 证书（HTTPS 是钉钉 OAuth 强制要求）。

### 2. 配置 `.env`
- 替换所有占位值为真实配置。
- 确保 `DINGTALK_APP_ID` 格式正确。
- 确保 `FRONTEND_URL` 和 `ALLOWED_ORIGINS` 为生产域名。

### 3. 配置钉钉后台
- 添加回调域名和授权回调地址。
- 确认应用已发布。

### 4. 启动服务
```bash
# 停止旧容器并强制重建（确保加载新环境变量）
docker-compose stop
docker-compose up -d --force-recreate

# 检查日志
docker-compose logs -f
```

### 5. 测试登录
- 访问 `https://<你的域名>/login`。
- 扫码测试钉钉登录。

---

## 四、待改进点（开发任务）

### 4.1 环境配置管理
- [ ] 支持多环境配置（dev/staging/prod），通过 `ENVIRONMENT` 变量切换。
- [ ] 区分钉钉/飞书凭证格式，增加启动时校验（若格式错误则 panic）。
- [ ] 提供 `.env.example` 模板和 `./scripts/check-env.sh` 配置检查工具。

### 4.2 自动迁移机制
- [ ] 在 `docker-compose.yml` 中添加 `init` 容器或 `entrypoint` 脚本自动执行 `sqlx migrate run`。
- [ ] 或集成 `sqlx migrate run` 到 Rust 启动流程（`main.rs` 启动前自动检查并迁移）。

### 4.3 前端环境变量
- [ ] 支持运行时环境变量注入（通过 Nginx 模板或 Docker entrypoint 生成 `config.json`）。
- [ ] 记录环境变量说明和默认值，提供配置中心（外部 JSON 文件）方案。

### 4.4 健康检查
- [ ] 添加 `/health` 端点，检查数据库、Redis、钉钉连接状态。
- [ ] 在 `docker-compose.yml` 中配置 `healthcheck`，实现自动重启故障容器。

---

## 五、常见问题速查

| 问题 | 原因 | 解决方法 |
| :--- | :--- | :--- |
| `relation "users" does not exist` | 数据库表未创建 | 执行 `sqlx migrate run` 或等待 init 容器完成 |
| `无效的 appid` | 使用了飞书 UUID 或占位值 | 检查 `.env`，确保使用 `cnxxx` 格式的钉钉 App ID |
| 回调域名不匹配 | 钉钉后台未配置或 `.env` 域名不一致 | 统一配置 `FRONTEND_URL` 并在钉钉后台添加回调 |
| 环境变量未生效 | 仅执行了 `docker-compose restart` | 执行 `docker-compose up -d --force-recreate` |
| 前端 API 地址错误 | Vite 构建时硬编码了 `localhost` | 使用运行时配置注入或重新构建前端镜像 |

---

**总结**：本次部署主要问题集中在 **环境配置管理** 和 **数据库初始化** 两个环节。建议开发团队优先完善自动迁移机制和环境配置校验，降低后续部署成本。
