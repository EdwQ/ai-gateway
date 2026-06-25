# Plan: API 调用内容采集与行为分析管道

## Phase 1: 数据模型设计与数据库迁移

- [x] Task: 设计 `call_contents` 表结构
    - [x] 定义字段：id, user_id, token_id, model, provider, request_content (JSONB), response_content (JSONB), file_metadata (JSONB), input_tokens, output_tokens, latency_ms, created_at, expires_at
    - [x] 设计 `content_masks` 表（存储脱敏规则命中记录）
    - [x] 设计索引策略（user_id + created_at, model, content_search via trigram）
- [x] Task: 创建数据库迁移 SQL
    - [x] 编写 `20240625000001_add_call_contents.sql`
    - [x] 编写 `20240625000002_add_content_masks.sql`
    - [ ] 测试迁移与回滚
- [x] Task: 编写 Rust 数据模型（models.rs）
    - [x] 定义 `CallContent` struct with serde derives
    - [x] 定义 `ContentMask` struct
    - [x] 实现 `Insertable` 和 `Queryable` trait 绑定
- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Rust 代理层内容拦截

- [x] Task: 实现请求内容拦截中间件
    - [x] 在 `proxy.rs` 的 `chat_completions` 处理函数中，在转发前提取请求 body
    - [x] 支持流式（SSE）和非流式两种模式的 body 提取
    - [x] 采集用户 ID、Token 指纹、模型名称等元数据
- [x] Task: 实现响应内容拦截
    - [x] 非流式：在收到上游响应后、返回客户端前，克隆 body 并提取内容
    - [x] 流式（SSE）：通过 `Stream` adapter 边转发边收集 chunks，重组完整响应
- [x] Task: 实现异步内容写入管道
    - [x] 使用 `tokio::spawn` 将采集数据发送到 mpsc channel
    - [x] 后台 consumer 批量写入 PostgreSQL（每批 100 条或每 500ms flush）
    - [x] 写入失败降级：记录日志但不阻塞代理请求
- [x] Task: 实现内容采集配置开关
    - [x] 读取 `CONTENT_CAPTURE_ENABLED` 环境变量
    - [x] 读取 `CONTENT_RETENTION_DAYS` 环境变量
    - [x] 通过 `config.rs` 注入配置到代理模块
- [ ] Task: 编写采集模块单元测试
    - [ ] Mock 请求/响应数据进行采集测试
    - [ ] 测试流式重组逻辑
    - [ ] 测试批量写入与降级逻辑
- [x] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: 数据聚合与过期清理

- [x] Task: 设计聚合表 `daily_usage_stats`
    - [x] 字段：date, user_id, model, provider, total_calls, total_input_tokens, total_output_tokens, total_cost, avg_latency_ms
- [x] Task: 实现聚合定时任务
    - [x] 使用 `tokio::spawn` 后台任务，按可配置间隔（默认每小时）运行
    - [x] 聚合查询使用 PostgreSQL 窗口函数
    - [x] 从 `call_contents` 扫描最近未聚合的数据
- [x] Task: 实现过期清理任务
    - [x] 定时扫描 `call_contents` 中 `expires_at < NOW()` 的记录
    - [x] 批量删除过期数据（每批 1000 条）
    - [x] 清理完成后更新统计信息
- [x] Task: 实现全文搜索接口
    - [x] 使用 PostgreSQL `pg_trgm` 扩展或 `to_tsvector` 实现
    - [x] 新增 REST 端点 `/api/v1/analysis/search`
    - [x] 分页返回结果，限制搜索范围在保留期内
- [x] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: 前端行为分析仪表盘

- [x] Task: 后端 API 路由实现
    - [x] `/api/v1/analysis/dashboard` — 概览指标
    - [x] `/api/v1/analysis/trends` — 时间序列数据
    - [x] `/api/v1/analysis/top-users` — 用户排行
    - [x] `/api/v1/analysis/top-models` — 模型排行
    - [x] `/api/v1/analysis/export` — CSV 导出
- [x] Task: 前端仪表盘页面
    - [x] 创建 `frontend/src/pages/Analysis/` 目录
    - [x] `Dashboard/index.tsx` — 概览指标卡片（调用量、Token、费用、延迟）
    - [x] `Trends/index.tsx` — 折线图（ECharts 或 Ant Design Charts）
    - [x] `Rankings/index.tsx` — 用户/模型排行 Table
- [x] Task: 导航与权限集成
    - [x] 在侧边栏添加 "行为分析" 菜单项
    - [x] 权限控制：仅 `admin+` 角色可访问
    - [x] 新增路由配置
- [ ] Task: 前端单元测试
    - [ ] 测试仪表盘组件渲染
    - [ ] Mock API 响应测试数据展示
- [x] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: 安全审计与异常检测

- [x] Task: 实现敏感数据检测引擎
    - [x] 基于 `mask.rs` 的现有 PII 脱敏逻辑扩展
    - [x] 正则匹配模式：API Key、密码、手机号、身份证号、银行卡号
    - [x] 命中结果写入 `content_masks` 表
- [x] Task: 实现异常检测
    - [x] 统计单位时间内单用户/单 IP 调用频率
    - [x] 检测非工作时间（如 0:00-6:00）的高频调用
    - [x] 检测短时间内同一 prompt 的重复调用
- [x] Task: 审计告警 API 与前端
    - [x] `/api/v1/analysis/alerts` — 告警列表
    - [ ] 前端告警页面 `Analysis/Alerts/index.tsx`
    - [ ] 告警级别标记（Info/Warning/Critical）
- [ ] Task: 集成测试
    - [ ] 端到端测试：请求 → 采集 → 存储 → 查询 → 展示
    - [ ] 性能测试：1000 并发请求下采集对延迟的影响 < 5ms
- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)

## Phase: Review Fixes
- [x] Task: Apply review suggestions 1411e5d
