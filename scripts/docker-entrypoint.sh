#!/bin/bash
# Docker entrypoint 脚本：启动前自动执行数据库迁移
set -e

echo "=== AI Gateway 启动脚本 ==="

# 检查是否需要执行迁移
if [ "${SKIP_MIGRATION:-false}" != "true" ]; then
  echo "检查数据库迁移..."
  
  # 等待数据库就绪
  echo "等待 PostgreSQL 就绪..."
  until pg_isready -h db -U postgres > /dev/null 2>&1; do
    echo -n "."
    sleep 2
  done
  echo " [数据库就绪]"
  
  # 尝试执行迁移
  echo "运行 SQLx 迁移..."
  if command -v sqlx > /dev/null 2>&1; then
    sqlx migrate run
  elif command -v cargo > /dev/null 2>&1; then
    cargo sqlx migrate run
  else
    echo "警告：未找到 sqlx 或 cargo，跳过迁移。请确保镜像中包含迁移工具。"
  fi
  
  echo "迁移完成。"
else
  echo "跳过迁移（SKIP_MIGRATION=true）。"
fi

# 检查关键环境变量
echo "检查关键环境变量..."
if [ -z "$DINGTALK_APP_ID" ] || [ "$DINGTALK_APP_ID" = "dev_app_id" ]; then
  echo "错误：DINGTALK_APP_ID 未配置或仍为占位值！"
  exit 1
fi

if [[ ! "$DINGTALK_APP_ID" =~ ^(cn|dinggq) ]]; then
  echo "警告：DINGTALK_APP_ID ($DINGTALK_APP_ID) 格式异常，通常应为 cnxxx 或 dinggqxxx 开头。"
fi

echo "环境变量检查通过。"

# 启动主服务
echo "启动 AI Gateway 服务..."
exec "$@"
