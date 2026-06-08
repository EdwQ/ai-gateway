import React, { useState, useEffect } from 'react';
import { Table, Select, Space, message } from 'antd';
import { getAuditLogs } from '../../api/client';

const actionOptions = [
  { label: '全部操作', value: '' },
  { label: '登录', value: 'login' },
  { label: '创建 Token', value: 'create_token' },
  { label: '删除 Token', value: 'delete_token' },
  { label: '更新用户', value: 'update_user' },
  { label: '创建 Provider', value: 'create_provider' },
  { label: '更新 Provider', value: 'update_provider' },
  { label: '删除 Provider', value: 'delete_provider' },
  { label: '查看 Prompt', value: 'view_prompt' },
];

export default function Audit() {
  const [logs, setLogs] = useState([]);
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [action, setAction] = useState('');

  const loadLogs = async () => {
    setLoading(true);
    try {
      const params: Record<string, unknown> = { page, page_size: 20 };
      if (action) params.action = action;
      const res = await getAuditLogs(params);
      setLogs(res.data.items || []);
      setTotal(res.data.total || 0);
    } catch {
      message.error('加载审计日志失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadLogs(); }, [page, action]);

  const columns = [
    { title: '操作', dataIndex: 'action', key: 'action', width: 120 },
    { title: '资源类型', dataIndex: 'resource_type', key: 'resource_type', width: 120 },
    { title: '资源 ID', dataIndex: 'resource_id', key: 'resource_id', ellipsis: true },
    { title: '用户 ID', dataIndex: 'user_id', key: 'user_id', ellipsis: true, width: 100 },
    { title: 'IP 地址', dataIndex: 'ip_address', key: 'ip_address', width: 140 },
    {
      title: '时间', dataIndex: 'created_at', key: 'created_at', width: 180,
      render: (v: string) => v ? new Date(v).toLocaleString() : '-',
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2>审计日志</h2>
        <Space>
          <Select
            style={{ width: 160 }}
            options={actionOptions}
            value={action}
            onChange={(v) => { setAction(v); setPage(1); }}
          />
        </Space>
      </div>

      <Table
        dataSource={logs}
        columns={columns}
        rowKey="id"
        loading={loading}
        pagination={{
          current: page,
          total,
          pageSize: 20,
          onChange: (p) => setPage(p),
          showTotal: (t) => `共 ${t} 条`,
        }}
      />
    </div>
  );
}
