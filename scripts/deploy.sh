#!/usr/bin/env bash
set -euo pipefail

# AI Gateway 一键部署脚本
# 功能：
# 1. 检查环境变量配置
# 2. 运行数据库迁移
# 3. 强制重建容器（确保加载新 .env）
# 4. 检查健康状态

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCKER_COMPOSE_FILE="$PROJECT_ROOT/docker-compose.yml"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# 检查 .env 文件
check_env() {
  log_info "检查环境变量配置..."
  
  if [[ ! -f "$PROJECT_ROOT/.env" ]]; then
    log_error ".env 文件不存在！请先复制 .env.example 并修改配置。"
    exit 1
  fi

  # 检查关键变量
  if grep -q "dev_app_id" "$PROJECT_ROOT/.env"; then
    log_error "发现占位值 'dev_app_id'，请替换为真实的钉钉 App ID！"
    exit 1
  fi

  if grep -q "your-secret-key" "$PROJECT_ROOT/.env" || grep -q "your-32-byte" "$PROJECT_ROOT/.env"; then
    log_error "发现默认密钥，请替换为真实的 SECRET_KEY 和 ENCRYPTION_KEY！"
    exit 1
  fi

  # 检查钉钉 App ID 格式
  DINGTALK_APP_KEY=$(grep "DINGTALK_APP_KEY=" "$PROJECT_ROOT/.env" | cut -d'=' -f2)
  if [[ ! "$DINGTALK_APP_KEY" =~ ^(cn|dinggq) ]]; then
    log_warn "钉钉 AppKey ($DINGTALK_APP_KEY) 格式异常，通常应为 cnxxx 或 dinggqxxx 开头。请确认是否误用了飞书的 UUID 格式。"
  fi

  log_info "环境变量检查通过。"
}

# 运行数据库迁移
run_migrations() {
  log_info "运行数据库迁移..."
  
  # 尝试使用 SQLx CLI 迁移
  if command -v sqlx &> /dev/null; then
    log_info "使用本地 sqlx CLI 执行迁移..."
    cd "$PROJECT_ROOT/backend-rs"
    sqlx migrate run
    cd "$PROJECT_ROOT"
  else
    log_info "在 Docker 容器中执行迁移..."
    docker-compose -f "$DOCKER_COMPOSE_FILE" exec -T backend sqlx migrate run || {
      # 如果容器内没有 sqlx，尝试使用 cargo 命令
      docker-compose -f "$DOCKER_COMPOSE_FILE" exec -T backend cargo sqlx migrate run || {
        log_warn "自动迁移失败，请手动执行：docker-compose exec backend sqlx migrate run"
        read -p "是否继续部署？(y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
          exit 1
        fi
      }
    }
  fi
  
  log_info "数据库迁移完成。"
}

# 停止并重建容器
recreate_containers() {
  log_info "停止旧容器并强制重建..."
  
  docker-compose -f "$DOCKER_COMPOSE_FILE" stop
  docker-compose -f "$DOCKER_COMPOSE_FILE" up -d --force-recreate
  
  log_info "容器重建完成。"
}

# 等待服务就绪
wait_for_services() {
  log_info "等待服务启动..."
  
  # 等待数据库就绪
  log_info "等待 PostgreSQL 就绪..."
  until docker-compose -f "$DOCKER_COMPOSE_FILE" exec -T db pg_isready -U postgres &> /dev/null; do
    echo -n "."
    sleep 2
  done
  echo " [OK]"
  
  # 等待 Redis 就绪
  log_info "等待 Redis 就绪..."
  until docker-compose -f "$DOCKER_COMPOSE_FILE" exec -T redis redis-cli ping &> /dev/null; do
    echo -n "."
    sleep 2
  done
  echo " [OK]"
  
  # 等待后端健康检查
  log_info "等待后端服务就绪..."
  for i in {1..30}; do
    if docker-compose -f "$DOCKER_COMPOSE_FILE" ps backend | grep -q "healthy\|Up"; then
      if curl -s http://localhost:3000/health/readiness > /dev/null 2>&1 || \
         docker-compose -f "$DOCKER_COMPOSE_FILE" exec -T backend curl -s http://localhost:8080/health/readiness > /dev/null 2>&1; then
        echo " [OK]"
        return 0
      fi
    fi
    echo -n "."
    sleep 3
  done
  
  log_warn "后端服务可能未完全就绪，请检查日志：docker-compose logs backend"
}

# 显示状态
show_status() {
  echo ""
  log_info "=== 服务状态 ==="
  docker-compose -f "$DOCKER_COMPOSE_FILE" ps
  echo ""
  log_info "=== 最近日志（后端）==="
  docker-compose -f "$DOCKER_COMPOSE_FILE" logs --tail=20 backend
}

# 主流程
main() {
  log_info "开始部署 AI Gateway..."
  
  check_env
  run_migrations
  recreate_containers
  wait_for_services
  show_status
  
  log_info "部署完成！请访问 https://apidashboard.surebestind.com 测试登录。"
  log_info "如需查看实时日志，运行：docker-compose logs -f"
}

# 处理参数
case "${1:-deploy}" in
  deploy)
    main
    ;;
  migrate)
    run_migrations
    ;;
  restart)
    recreate_containers
    ;;
  status)
    show_status
    ;;
  *)
    echo "用法：$0 {deploy|migrate|restart|status}"
    exit 1
    ;;
esac
