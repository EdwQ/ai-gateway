import React, { useState, useEffect } from 'react';
import { Table, Button, Modal, Form, Input, InputNumber, Tag, Space, message, Tooltip, Select } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, HeartOutlined } from '@ant-design/icons';
import { getProviders, createProvider, updateProvider, deleteProvider, checkProviderHealth, discoverModels } from '../../api/client';

const statusColors: Record<string, string> = {
  healthy: 'green',
  degraded: 'orange',
  down: 'red',
  unknown: 'default',
};

const statusLabels: Record<string, string> = {
  healthy: '健康',
  degraded: '亚健康',
  down: '不可用',
  unknown: '未知',
};

export default function Providers() {
  const [providers, setProviders] = useState([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<any>(null);
  const [form] = Form.useForm();

  const loadProviders = async () => {
    setLoading(true);
    try {
      const res = await getProviders();
      setProviders(res.data.items || []);
    } catch {
      message.error('加载 Provider 列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadProviders(); }, []);

  const handleAdd = () => {
    setEditingProvider(null);
    form.resetFields();
    setModalOpen(true);
  };

  const handleEdit = (provider: any) => {
    setEditingProvider(provider);
    form.setFieldsValue(provider);
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      if (editingProvider) {
        await updateProvider(editingProvider.id, values);
        message.success('Provider 已更新');
      } else {
        await createProvider(values);
        message.success('Provider 已创建');
      }
      setModalOpen(false);
      loadProviders();
    } catch (err: any) {
      if (err.response) {
        message.error(err.response.data?.detail || '操作失败');
      }
    }
  };

  const handleDelete = (id: string) => {
    Modal.confirm({
      title: '确认删除',
      content: '删除后不可恢复，确定继续？',
      okText: '确定', cancelText: '取消',
      onOk: async () => {
        try {
          await deleteProvider(id);
          message.success('Provider 已删除');
          loadProviders();
        } catch { message.error('删除失败'); }
      },
    });
  };

  const handleHealthCheck = async (id: string) => {
    try {
      const res = await checkProviderHealth(id);
      message.info(`状态: ${res.data.status} (延迟: ${res.data.latency_ms}ms)`);
      loadProviders();
    } catch {
      message.error('健康检查失败');
    }
  };

  const handleDiscoverModels = async () => {
    const baseUrl = form.getFieldValue('base_url');
    const apiKey = form.getFieldValue('api_key');
    if (!baseUrl || !apiKey) {
      message.warning('请先填写 Base URL 和 API Key');
      return;
    }
    try {
      const res = await discoverModels(baseUrl, apiKey);
      const models = res.data.models || [];
      if (models.length === 0) {
        message.warning(`未获取到模型列表${res.data.error ? '：' + res.data.error : ''}`);
        return;
      }
      form.setFieldsValue({ models });
      message.success(`获取到 ${models.length} 个模型`);
    } catch (err: any) {
      message.error(err.response?.data?.detail || '获取模型列表失败');
    }
  };

  const columns = [
    { title: '名称', dataIndex: 'display_name', key: 'display_name' },
    {
      title: '状态', dataIndex: 'health_status', key: 'health_status',
      render: (v: string) => <Tag color={statusColors[v] || 'default'}>{statusLabels[v] || v}</Tag>,
    },
    { title: '模型', dataIndex: 'models', key: 'models', render: (v: string[]) => (v || []).join(', ') },
    { title: '优先级', dataIndex: 'priority', key: 'priority' },
    { title: 'QPS', dataIndex: 'rate_limit_qps', key: 'rate_limit_qps' },
    {
      title: '操作', key: 'actions',
      render: (_: any, record: any) => (
        <Space>
          <Tooltip title="健康检查">
            <Button type="link" icon={<HeartOutlined />} onClick={() => handleHealthCheck(record.id)} />
          </Tooltip>
          <Tooltip title="编辑">
            <Button type="link" icon={<EditOutlined />} onClick={() => handleEdit(record)} />
          </Tooltip>
          <Tooltip title="删除">
            <Button type="link" danger icon={<DeleteOutlined />} onClick={() => handleDelete(record.id)} />
          </Tooltip>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 16, display: 'flex', justifyContent: 'space-between' }}>
        <h2>Provider 管理</h2>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增 Provider</Button>
      </div>

      <Table dataSource={providers} columns={columns} rowKey="id" loading={loading} />

      <Modal
        title={editingProvider ? '编辑 Provider' : '新增 Provider'}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
        okText="保存"
        cancelText="取消"
        width={600}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true }]}>
            <Input placeholder="openai" disabled={!!editingProvider} />
          </Form.Item>
          <Form.Item name="display_name" label="显示名称" rules={[{ required: true }]}>
            <Input placeholder="OpenAI" />
          </Form.Item>
          <Form.Item name="base_url" label="API Base URL" rules={[{ required: true }]}>
            <Input placeholder="https://api.openai.com" />
          </Form.Item>
          <Form.Item name="api_key" label="API Key" rules={editingProvider ? [] : [{ required: true }]}>
            <Input.Password placeholder={editingProvider ? '留空则不修改' : 'sk-...'} />
          </Form.Item>
          <Form.Item label="模型列表">
            <Space.Compact block>
              <Form.Item name="models" noStyle>
                <Select mode="tags" placeholder="输入模型名后回车" style={{ flex: 1 }} />
              </Form.Item>
              <Tooltip title="从 Provider 自动获取模型列表">
                <Button icon={<HeartOutlined />} onClick={handleDiscoverModels}>
                  获取模型
                </Button>
              </Tooltip>
            </Space.Compact>
          </Form.Item>
          <Form.Item name="priority" label="优先级">
            <InputNumber min={1} max={999} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="rate_limit_qps" label="QPS 限制">
            <InputNumber min={1} max={10000} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
