# 开发任务：部署改进与自动化

> **目标**：解决首次部署中的配置错误、迁移缺失和环境变量问题，降低部署成本。

---

## 🎯 优先级：高

### 1. 自动数据库迁移 ✅ 已完成部分
**状态**: 进行中  
**文件**: `scripts/docker-entrypoint.sh`, `docker-compose.yml`

**已完成**:
- [x] 创建 `docker-entrypoint.sh` 启动脚本
- [x] 在 `docker-compose.yml` 中添加健康检查和依赖条件
- [x] 配置 `sqlx migrate run` 自动执行

**待完成**:
- [ ] 确保 Docker 镜像中包含 `sqlx` CLI 或 `cargo sqlx` 工具
- [ ] 测试首次部署时自动迁移是否成功
- [ ] 添加迁移失败时的优雅降级处理

**验收标准**:
- 新部署的容器能自动执行数据库迁移，无需手动干预
- 迁移失败时服务不启动并输出明确错误信息

---

### 2. 配置校验工具 ✅ 已完成部分
**状态**: 进行中  
**文件**: `scripts/deploy.sh`

**已完成**:
- [x] 创建 `deploy.sh` 脚本，包含环境变量检查
- [x] 检测钉钉 App ID 格式（cnxxx/dinggqxxx）
- [x] 检测占位值（如 `dev_app_id`, `your-secret-key`）
- [x] 强制重建容器（`--force-recreate`）

**待完成**:
- [ ] 添加 `.env.example` 模板，明确标注必填项
- [ ] 增加配置检查工具 `./scripts/check-env.sh`（独立于部署）
- [ ] 支持多环境配置（dev/staging/prod）

**验收标准**:
- 配置错误时脚本提前报错，避免部署后发现问题
- 提供清晰的错误提示和修复建议

---

### 3. 钉钉凭证格式校验
**状态**: 待开发  
**文件**: `backend-rs/src/core/config.rs` 或启动检查

**需求**:
- 在应用启动时校验 `DINGTALK_APP_ID` 格式
- 若格式不符合钉钉规范（非 `cn`/`dinggq` 开头），直接 panic 并输出明确错误
- 区分钉钉和飞书凭证，避免混淆

**验收标准**:
- 使用飞书 UUID 格式的钉钉 App ID 时，服务启动失败并提示正确格式

---

### 4. 前端运行时配置
**状态**: 待开发  
**文件**: `frontend/`, `docker-compose.yml`, Nginx 配置

**问题**: Vite 构建时硬编码环境变量，运行时无法修改。

**可选方案**:
1. **Nginx 模板替换**：通过 `envsubst` 动态生成 `index.html`
2. **外部配置文件**：启动时请求 `/config.json` 获取 API 地址
3. **Docker entrypoint 注入**：生成 `window.APP_CONFIG` 对象

**待完成**:
- [ ] 选择并实现运行时配置方案
- [ ] 修改前端代码支持动态配置
- [ ] 更新 Docker 构建流程

**验收标准**:
- 修改 `.env` 后，无需重新构建前端镜像即可生效

---

## 🎯 优先级：中

### 5. 健康检查端点增强
**状态**: 待开发  
**文件**: `backend-rs/src/api/v1/health.rs`

**需求**:
- 现有 `/health/readiness` 仅检查服务本身
- 需要增加对数据库、Redis、钉钉连接的深度检查

**待完成**:
- [ ] 添加 `/health/db` 检查数据库连接
- [ ] 添加 `/health/redis` 检查 Redis 连接
- [ ] 添加 `/health/dingtalk` 检查钉钉 API 连通性
- [ ] 在 `docker-compose.yml` 中配置健康检查

**验收标准**:
- Docker 能根据健康检查状态自动重启故障容器
- Grafana/Prometheus 可监控各依赖服务状态

---

### 6. 多环境配置支持
**状态**: 待开发  
**文件**: `.env.dev`, `.env.staging`, `.env.prod`

**需求**:
- 分离开发、测试、生产环境配置
- 通过 `ENVIRONMENT` 变量切换

**待完成**:
- [ ] 创建 `.env.dev`, `.env.staging`, `.env.prod` 模板
- [ ] 修改 `deploy.sh` 支持 `--env` 参数
- [ ] 添加环境配置校验脚本

**验收标准**:
- 可通过 `./scripts/deploy.sh --env=prod` 一键部署到不同环境

---

## 🎯 优先级：低

### 7. 前端配置中心
**状态**: 待开发  
**文件**: 外部配置服务或 Nginx 配置

**需求**:
- 将前端配置（API 地址、功能开关等）分离到外部 JSON/YAML 文件
- 前端运行时动态加载配置

**待完成**:
- [ ] 设计配置 JSON 结构
- [ ] 实现前端动态加载逻辑
- [ ] 提供配置管理界面（可选）

---

### 8. 日志聚合与监控
**状态**: 待开发  
**文件**: `prometheus/`, `grafana/`

**需求**:
- 完善 Prometheus 指标采集
- 配置 Grafana 监控大盘
- 设置告警规则

**待完成**:
- [ ] 添加业务指标（API 调用次数、错误率、响应时间）
- [ ] 配置 Grafana 大盘模板
- [ ] 设置钉钉/邮件告警

---

## 📋 已完成的工作

- [x] 编写 `DEPLOYMENT_TROUBLESHOOTING.md` 部署问题总结文档
- [x] 创建 `scripts/deploy.sh` 自动化部署脚本
- [x] 创建 `scripts/docker-entrypoint.sh` 启动迁移脚本
- [x] 更新 `docker-compose.yml` 添加健康检查和依赖条件
- [x] 更新 `README.md` 添加部署提示和新文档引用
- [x] 更新 `AI_Gateway_MVP_Plan.md` 将 Python 内容标记为过时

---

## 🚀 下一步行动

1. **立即执行**：测试 `./scripts/deploy.sh` 在真实环境中的表现
2. **本周内**：确保 Docker 镜像包含 `sqlx` 迁移工具
3. **下周**：实现前端运行时配置方案
4. **持续**：根据实际部署反馈迭代改进

---

**备注**：所有改进旨在降低部署复杂度，避免重复踩坑。优先保证**自动化**和**错误预判**，减少人工干预。
