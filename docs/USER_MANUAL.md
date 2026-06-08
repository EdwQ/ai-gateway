# AI Gateway 用户使用手册

> **版本**: v1.0  
> **适用范围**: 企业内部 AI 能力平台  
> **最后更新**: 2026-06-07

---

## 目录

- [1. 系统概述](#1-系统概述)
- [2. 快速上手](#2-快速上手)
- [3. 登录与账户](#3-登录与账户)
- [4. API Token 管理](#4-api-token-管理)
- [5. 调用 AI 接口](#5-调用-ai-接口)
- [6. Provider 管理（管理员）](#6-provider-管理管理员)
- [7. 用户管理（管理员）](#7-用户管理管理员)
- [8. 数据统计与报表](#8-数据统计与报表)
- [9. 监控与运维](#9-监控与运维)
- [10. 常见问题](#10-常见问题)

---

## 1. 系统概述

### 1.1 什么是 AI Gateway

AI Gateway 是一个企业内部的统一 AI API 网关，提供以下核心能力：

- **统一入口**：通过一个网关对接多个 AI 服务商（OpenAI、DeepSeek、通义千问、Claude 等）
- **OpenAI 兼容**：完全兼容 OpenAI SDK，现有代码无需改造即可接入
- **成本控制**：实时统计 Token 消耗，支持按用户/部门额度管理
- **审计安全**：完整的操作审计日志和 Prompt 脱敏保存
- **SSO 登录**：支持钉钉扫码登录，自动开户

### 1.2 架构图

```
┌─────────────────────┐     ┌──────────────────────┐
│  前端管理后台        │     │  AI 客户端工具         │
│  (localhost:3000)   │     │  (Cursor / ChatBox    │
│                     │     │   / 自定义应用)        │
└────────┬────────────┘     └──────────┬───────────┘
         │                             │
         ▼                             ▼
┌──────────────────────────────────────────────┐
│              AI Gateway API                   │
│          (http://localhost:8010)              │
├──────────────────────────────────────────────┤
│  认证 → 限流 → 路由 → 转发 → 审计 → 计费    │
└───────┬──────────────────────────┬───────────┘
        │                          │
        ▼                          ▼
┌──────────────┐       ┌──────────────────┐
│  PostgreSQL   │       │     Redis        │
│  (用户/账单)  │       │   (限流/缓存)    │
└──────────────┘       └──────────────────┘
        │
        ▼
┌──────────────────────────────────────────────┐
│          上游 AI Provider                     │
│  OpenAI / DeepSeek / 通义千问 / Claude / ... │
└──────────────────────────────────────────────┘
```

### 1.3 服务地址

部署完成后，通过以下地址访问各项服务：

| 服务 | 地址 | 说明 |
|:---|:---|:---|
| **前端管理后台** | http://localhost:3000 | 管理界面入口 |
| **API 接口** | http://localhost:8010 | AI 网关接口 |
| **API 文档** | http://localhost:8010/docs | Swagger 交互式文档 |
| **Prometheus** | http://localhost:9090 | 指标数据 |
| **Grafana** | http://localhost:3001 | 监控面板（admin/admin） |

---

## 2. 快速上手

### 2.1 5 分钟快速体验

```bash
# 1. 验证服务正常运行
curl http://localhost:8010/health/liveness

# 2. 查看可用模型列表（无需认证）
curl http://localhost:8010/v1/models

# 3. 在浏览器打开管理后台
open http://localhost:3000
```

### 2.2 典型使用流程

```
注册/登录 → 创建 API Token → 配置客户端 → 开始调用
   ↓            ↓               ↓              ↓
钉钉扫码    生成 Key      OpenAI SDK      AI 对话
首次自动    保存 Key      设置 base_url   自动计费
开户赠额    仅显示一次    和 api_key     实时统计
```

---

## 3. 登录与账户

### 3.1 钉钉扫码登录

> **前提**：管理员已在 `.env` 中配置钉钉应用参数。

1. 访问 http://localhost:3000
2. 点击「钉钉登录」按钮
3. 使用钉钉手机端扫描二维码
4. 首次登录自动创建账户，并获得 **50 元默认额度**

### 3.2 账户信息

登录后，在首页可查看：

- **姓名 / 部门**：来自钉钉组织架构
- **角色**：`employee`（普通员工）/ `admin`（管理员）/ `super_admin`（超级管理员）
- **额度余额**：当前可用额度（元）
- **已用额度**：本月已消耗额度
- **API 调用次数**：今日调用统计

### 3.3 额度说明

- 新用户默认获得 **50 元** 额度
- 管理员可在后台调整用户额度
- 额度不足时，调用 AI 接口会返回 `402 Payment Required`
- 额度计算基于模型价格表，按 Token 使用量计费

---

## 4. API Token 管理

### 4.1 创建 Token

1. 登录管理后台
2. 左侧菜单点击「API Token」
3. 点击「创建 Token」
4. 输入名称（可选，方便管理）
5. 点击确定

### 4.2 重要：保存 Token

> ⚠️ **Token 仅在创建时显示一次**，关闭弹窗后无法再次查看原文。

创建成功后，请立即：

```bash
# 复制 Token 并保存在安全位置（如密码管理器）
sk-company-a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6a7b8c9d0e1f
```

### 4.3 Token 管理操作

| 操作 | 说明 |
|:---|:---|
| **创建** | 生成一个新的 API Key |
| **查看列表** | 查看所有 Token 的名称、创建时间、最后使用时间 |
| **轮换** | 撤销旧 Token 并生成新 Token（不影响正在处理的请求） |
| **删除** | 永久撤销 Token，立即失效 |

### 4.4 安全建议

- 定期轮换 Token（建议每 90 天）
- 不要在代码中硬编码 Token
- 使用环境变量注入 Token
- 不同应用使用不同 Token，方便审计

---

## 5. 调用 AI 接口

### 5.1 OpenAI 兼容 SDK

AI Gateway 提供完全兼容 OpenAI 的 API，使用 OpenAI SDK 即可直接调用：

**Python 示例：**

```python
from openai import OpenAI

client = OpenAI(
    api_key="sk-company-xxxxx...",       # 替换为你的 Token
    base_url="http://localhost:8010/v1"  # 网关地址
)

# 列出可用模型
models = client.models.list()
print("可用模型:", [m.id for m in models])

# Chat 对话
response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {"role": "system", "content": "你是一个 helpful assistant。"},
        {"role": "user", "content": "你好，请介绍一下你自己"}
    ]
)
print(response.choices[0].message.content)
```

**Node.js 示例：**

```javascript
import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'sk-company-xxxxx...',
  baseURL: 'http://localhost:8010/v1'
});

const response = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: [
    { role: 'user', content: 'Hello!' }
  ]
});

console.log(response.choices[0].message.content);
```

**cURL 示例：**

```bash
curl http://localhost:8010/v1/chat/completions \
  -H "Authorization: Bearer sk-company-xxxxx..." \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### 5.2 流式响应

支持 SSE（Server-Sent Events）流式输出，适用于打字机效果：

```python
stream = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "讲个故事"}],
    stream=True
)
for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="", flush=True)
```

### 5.3 在第三方工具中使用

AI Gateway 兼容主流 AI 工具，配置方式：

**Cursor：**
```
设置 → Models → OpenAI API Key → 填入你的 Token
OpenAI Base URL → http://localhost:8010/v1
```

**Cherry Studio：**
```
设置 → AI 服务 → OpenAI → 
API 地址: http://localhost:8010/v1
API Key: 你的 Token
```

**Open WebUI：**
```
管理员面板 → 设置 → 外部连接 → OpenAI API →
API URL: http://localhost:8010/v1
API Key: 你的 Token
```

### 5.4 Embeddings 接口

```python
response = client.embeddings.create(
    model="text-embedding-ada-002",
    input="这是一段需要向量化的文本"
)
print(response.data[0].embedding)
```

### 5.5 可用模型列表

```bash
curl http://localhost:8010/v1/models \
  -H "Authorization: Bearer sk-company-xxxxx..."
```

返回示例：

```json
{
  "data": [
    {"id": "gpt-4o", "object": "model", ...},
    {"id": "deepseek-chat", "object": "model", ...},
    {"id": "claude-3-opus", "object": "model", ...}
  ]
}
```

> 模型列表由管理员在 Provider 管理中配置，实际可用模型取决于已添加的 Provider。

---

## 6. Provider 管理（管理员）

### 6.1 什么是 Provider

Provider 是上游 AI 服务商的配置。一个 Provider 对应一个 API 服务地址（如 OpenAI、DeepSeek）。

### 6.2 添加 Provider

1. 使用管理员账号登录
2. 左侧菜单进入「Provider 管理」
3. 点击「新增 Provider」
4. 填写以下信息：

| 字段 | 说明 | 示例 |
|:---|:---|:---|
| 名称 | 内部标识（英文） | `deepseek` |
| 显示名称 | 前端展示名称 | `DeepSeek` |
| API Base URL | 服务商接口地址 | `https://api.deepseek.com` |
| API Key | 你的服务商密钥 | `sk-xxx` |
| 模型列表 | 该 Provider 支持的模型 | `deepseek-chat, deepseek-reasoner` |
| 优先级 | 数字越小优先级越高 | `100` |
| QPS 限制 | 每秒请求数上限 | `60` |

### 6.3 支持的 Provider 模板

| Provider | API Base URL |
|:---|:---|
| OpenAI | `https://api.openai.com` |
| Anthropic Claude | `https://api.anthropic.com` |
| Google Gemini | `https://generativelanguage.googleapis.com` |
| DeepSeek | `https://api.deepseek.com` |
| 通义千问 | `https://dashscope.aliyuncs.com` |
| Ollama（本地） | `http://host.docker.internal:11434` |
| vLLM（本地） | `http://host.docker.internal:8000` |

### 6.4 健康检查

添加 Provider 后，可点击「健康检查」按钮测试连通性：

```json
// 成功响应
{"status": "healthy", "latency_ms": 234}

// 失败响应
{"status": "unhealthy", "error": "Connection refused"}
```

### 6.5 多 Key 负载均衡

支持为每个 Provider 配置多个 API Key，系统自动轮询和故障切换：

```
Provider: DeepSeek
  Key 1: sk-aaa (优先级 100) ← 优先使用
  Key 2: sk-bbb (优先级 100) ← 故障时自动切换
  Key 3: sk-ccc (优先级 200) ← 备用
```

---

## 7. 用户管理（管理员）

### 7.1 用户列表

管理员可在「用户管理」页面查看所有用户，包含：

- 姓名、部门、邮箱（来自钉钉同步）
- 角色（employee / admin / super_admin / finance）
- 账户状态（启用/禁用）
- 额度余额
- 本月已用额度
- 注册时间

### 7.2 用户操作

| 操作 | 说明 |
|:---|:---|
| 编辑用户 | 修改角色、额度 |
| 启用/禁用 | 禁用后用户无法调用 API |
| 充值额度 | 增加用户可用额度 |
| 查看详情 | 查看用户调用历史和消费记录 |

### 7.3 角色权限

| 角色 | 查看统计 | 管理 Token | 管理用户 | 管理 Provider | 导出报表 |
|:---|:---:|:---:|:---:|:---:|:---:|
| employee | 自己的 | ✅ | ❌ | ❌ | ❌ |
| admin | 全部 | ✅ | ✅ | ✅ | ✅ |
| super_admin | 全部 | ✅ | ✅ | ✅ | ✅ |
| finance | 全部 | ❌ | ❌ | ❌ | ✅ |

---

## 8. 数据统计与报表

### 8.1 仪表盘

登录后首页展示关键指标：

- **今日调用次数**
- **本月总消耗（元）**
- **本日 Token 消耗**
- **本月 Token 消耗趋势图**
- **各模型调用分布**
- **各用户调用排行**

### 8.2 数据统计

进入「数据统计」页面：

- **日报**：按天查看 Token 消耗和费用
- **月报**：按月查看汇总数据
- **趋势图**：可视化展示使用趋势

### 8.3 报表导出

```bash
# 导出指定月份的 CSV 报表
curl http://localhost:8010/api/v1/stats/export?month=2026-06 \
  -H "Authorization: Bearer <admin_token>"
```

导出的 CSV 包含：

| 字段 | 说明 |
|:---|:---|
| 日期 | 调用日期 |
| 用户 | 调用者 |
| 模型 | 使用的模型 |
| Token 输入 | 输入 Token 数 |
| Token 输出 | 输出 Token 数 |
| 费用（元） | 本次调用费用 |

### 8.4 手动查看指标

后端暴露 Prometheus 指标：

```bash
curl http://localhost:8010/metrics
```

关键指标：

| 指标名 | 类型 | 说明 |
|:---|:---|:---|
| `ai_gateway_requests_total` | Counter | 请求总数 |
| `ai_gateway_request_duration_ms` | Histogram | 请求延迟 |
| `ai_gateway_sse_streams_total` | Counter | SSE 流数 |
| `ai_gateway_tokens_total` | Counter | Token 用量 |

---

## 9. 监控与运维

### 9.1 健康检查

API 提供了两个健康检查端点：

```bash
# Liveness Probe（存活检查）
curl http://localhost:8010/health/liveness
# → {"status": "alive", "timestamp": ...}

# Readiness Probe（就绪检查）
curl http://localhost:8010/health/readiness
# → {"status": "ready", "database": "ok", "redis": "ok", ...}
```

### 9.2 查看日志

```bash
# 查看所有服务日志
docker compose logs -f

# 查看特定服务日志
docker compose logs -f backend
docker compose logs -f frontend

# 查看最近 100 行
docker compose logs --tail=100 backend
```

### 9.3 Grafana 监控面板

访问 http://localhost:3001 （默认账号：admin / admin）

预置面板包含：

- **请求量趋势**：实时 API 调用次数
- **延迟分布**：各接口响应时间
- **错误率**：4xx/5xx 错误统计
- **Token 使用量**：各模型 Token 消耗

### 9.4 Prometheus 指标

访问 http://localhost:9090 可执行 PromQL 查询：

```promql
# 最近 5 分钟请求速率
rate(ai_gateway_requests_total[5m])

# 平均延迟
avg(ai_gateway_request_duration_ms)

# 各模型 Token 消耗
sum by(model) (ai_gateway_tokens_total)
```

### 9.5 审计日志

系统自动记录以下敏感操作：

| 操作类型 | 记录内容 |
|:---|:---|
| 登录 | 登录时间、IP、来源 |
| Token 操作 | 创建、轮换、删除 |
| 用户管理 | 修改角色、额度变更 |
| Provider 变更 | 新增、修改、删除 Provider |

登录管理后台 → 「审计日志」可查看和筛选。

### 9.6 日志脱敏

系统自动对日志中的敏感信息进行脱敏处理：

| 类型 | 原文示例 | 脱敏后 |
|:---|:---|:---|
| Authorization | `Bearer sk-company-xxx` | `Bearer ***` |
| 手机号 | `13800138000` | `138****8000` |
| 身份证 | `110101199001011234` | `110101********1234` |
| 邮箱 | `zhangsan@company.com` | `z****n@company.com` |

---

## 10. 常见问题

### 10.1 接口返回 401 Unauthorized

**原因**：Token 无效或已过期。

**解决**：
1. 检查 Token 是否已复制完整（包括 `sk-company-` 前缀）
2. 在管理后台重新创建 Token
3. 确认 Token 未被删除或轮换

### 10.2 接口返回 402 Payment Required

**原因**：账户额度不足。

**解决**：
1. 登录管理后台查看当前额度
2. 联系管理员充值

### 10.3 接口返回 429 Too Many Requests

**原因**：超出 QPS 限制（默认用户 10 QPS，Provider 100 QPS）。

**解决**：
1. 降低请求频率
2. 如需提高限制，联系管理员调整配置

### 10.4 提示"模型不可用"

**原因**：请求的模型未配置或 Provider 故障。

**解决**：
1. 先调用 `GET /v1/models` 查看可用模型列表
2. 联系管理员确认 Provider 是否正常运行
3. 管理员可在 Provider 管理页面执行健康检查

### 10.5 钉钉登录失败

**原因**：钉钉应用配置不正确。

**解决**：
1. 确认 `.env` 中的 `DINGTALK_APP_ID` 和 `DINGTALK_APP_SECRET` 已正确配置
2. 确认钉钉应用已开启扫码登录权限
3. 确认回调域名正确配置

### 10.6 忘记管理员密码

**原因**：AI Gateway 使用钉钉 SSO，没有独立密码。

**解决**：
1. 用钉钉扫码登录即可
2. 如果钉钉账号无法登录，联系服务器管理员在数据库中修改角色

---

## 附录 A：环境变量参考

| 变量名 | 默认值 | 说明 |
|:---|:---|:---|
| `SECRET_KEY` | - | JWT 签名密钥（必填，至少 32 字符） |
| `ENCRYPTION_KEY` | - | AES-256 加密密钥（必填，32 字节） |
| `DATABASE_URL` | `postgresql+asyncpg://...` | 数据库连接串 |
| `REDIS_URL` | `redis://redis:6379/0` | Redis 连接串 |
| `DINGTALK_APP_ID` | - | 钉钉应用 AppKey |
| `DINGTALK_APP_SECRET` | - | 钉钉应用 AppSecret |
| `DEBUG` | `false` | 调试模式 |
| `DEFAULT_QUOTA_AMOUNT` | `50.0` | 新用户默认额度 |
| `RATE_LIMIT_USER_QPS` | `10` | 用户 QPS 上限 |
| `RATE_LIMIT_PROVIDER_QPS` | `100` | Provider QPS 上限 |
| `PROMPT_SAVE_MODE` | `off` | Prompt 保存策略 |

## 附录 B：部署运维命令

```bash
# 启动所有服务
docker compose up -d

# 停止所有服务
docker compose down

# 重启特定服务
docker compose restart backend

# 查看实时日志
docker compose logs -f backend

# 重新构建并启动（代码更新后）
docker compose up -d --build backend

# 重置所有数据（谨慎！会清空数据库）
docker compose down -v
docker compose up -d
```
