import React, { useState, useEffect } from 'react';
import { Table, Button, Modal, Form, Select, InputNumber, Tag, Space, message, Input } from 'antd';
import { EditOutlined, SearchOutlined } from '@ant-design/icons';
import { getUsers, updateUser, getAliases } from '../../api/client';

const roleLabels: Record<string, string> = {
  employee: '员工',
  admin: '管理员',
  finance: '财务',
  super_admin: '超级管理员',
};

export default function Users() {
  const [users, setUsers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [editModal, setEditModal] = useState(false);
  const [editingUser, setEditingUser] = useState<any>(null);
  const [allAliases, setAllAliases] = useState<any[]>([]);
  const [form] = Form.useForm();

  const loadUsers = async () => {
    setLoading(true);
    try {
      const params: Record<string, unknown> = { page, page_size: 20 };
      if (search) params.search = search;
      const res = await getUsers(params);
      setUsers(res.data.items || []);
      setTotal(res.data.total || 0);
    } catch {
      message.error('加载用户列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadUsers(); }, [page]);

  // Load available model aliases for the allowed_models selector
  useEffect(() => {
    getAliases().then(res => {
      setAllAliases(res.data.items || []);
    }).catch(() => {});
  }, []);

  const handleEdit = (user: any) => {
    setEditingUser(user);
    form.setFieldsValue(user);
    setEditModal(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      await updateUser(editingUser.id, values);
      message.success('用户信息已更新');
      setEditModal(false);
      loadUsers();
    } catch (err: any) {
      if (err.response) message.error(err.response.data?.detail || '更新失败');
    }
  };

  const columns = [
    { title: '姓名', dataIndex: 'name', key: 'name' },
    { title: '邮箱', dataIndex: 'email', key: 'email', render: (v: string) => v || '-' },
    { title: '部门', dataIndex: 'department_name', key: 'department_name', render: (v: string) => v || '-' },
    {
      title: '角色', dataIndex: 'role', key: 'role',
      render: (v: string) => <Tag>{roleLabels[v] || v}</Tag>,
    },
    {
      title: '状态', dataIndex: 'is_active', key: 'is_active',
      render: (v: boolean) => v ? <Tag color="green">启用</Tag> : <Tag color="red">禁用</Tag>,
    },
    { title: '可用模型', dataIndex: 'allowed_models', key: 'allowed_models',
      render: (v: string[]) => (v || []).length ? v.join(', ') : '全部' },
    { title: '额度 (¥)', dataIndex: 'quota_balance', key: 'quota_balance', render: (v: number) => `¥${Number(v ?? 0).toFixed(2)}` },
    { title: '已用 (¥)', dataIndex: 'quota_used', key: 'quota_used', render: (v: number) => `¥${Number(v ?? 0).toFixed(2)}` },
    {
      title: '操作', key: 'actions',
      render: (_: any, record: any) => (
        <Button type="link" icon={<EditOutlined />} onClick={() => handleEdit(record)}>编辑</Button>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2>用户管理</h2>
        <Input
          placeholder="搜索用户姓名/邮箱/部门"
          prefix={<SearchOutlined />}
          style={{ width: 300 }}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onPressEnter={() => { setPage(1); loadUsers(); }}
        />
      </div>

      <Table
        dataSource={users}
        columns={columns}
        rowKey="id"
        loading={loading}
        pagination={{
          current: page,
          total,
          pageSize: 20,
          onChange: (p) => setPage(p),
          showTotal: (t) => `共 ${t} 人`,
        }}
      />

      <Modal
        title="编辑用户"
        open={editModal}
        onOk={handleSave}
        onCancel={() => setEditModal(false)}
        okText="保存"
        cancelText="取消"
      >
        <Form form={form} layout="vertical">
          <Form.Item name="role" label="角色">
            <Select
              options={[
                { label: '员工', value: 'employee' },
                { label: '管理员', value: 'admin' },
                { label: '财务', value: 'finance' },
                { label: '超级管理员', value: 'super_admin' },
              ]}
            />
          </Form.Item>
          <Form.Item name="is_active" label="状态">
            <Select
              options={[
                { label: '启用', value: true },
                { label: '禁用', value: false },
              ]}
            />
          </Form.Item>
          <Form.Item name="quota_balance" label="额度 (¥)">
            <InputNumber min={0} step={10} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="allowed_models" label="可用模型（留空则不限制）">
            <Select mode="tags" placeholder="选择用户可用的模型别名">
              {allAliases.filter(a => a.is_active).map(a => (
                <Select.Option key={a.alias_name} value={a.alias_name}>
                  {a.alias_name} → {a.target_model}
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
