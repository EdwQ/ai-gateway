import React, { useState, useEffect } from 'react';
import { Table, Button, Modal, Form, Input, Switch, Tag, Space, message, Tooltip } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
import { getAliases, createAlias, updateAlias, deleteAlias } from '../../api/client';

export default function ModelAliases() {
  const [aliases, setAliases] = useState([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingAlias, setEditingAlias] = useState<any>(null);
  const [form] = Form.useForm();

  const loadAliases = async () => {
    setLoading(true);
    try {
      const res = await getAliases();
      setAliases(res.data.items || []);
    } catch {
      message.error('加载模型别名列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadAliases(); }, []);

  const handleAdd = () => {
    setEditingAlias(null);
    form.resetFields();
    setModalOpen(true);
  };

  const handleEdit = (alias: any) => {
    setEditingAlias(alias);
    form.setFieldsValue(alias);
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      if (editingAlias) {
        await updateAlias(editingAlias.id, values);
        message.success('别名已更新');
      } else {
        await createAlias(values);
        message.success('别名已创建');
      }
      setModalOpen(false);
      loadAliases();
    } catch (err: any) {
      if (err.response) {
        message.error(err.response.data?.detail || '操作失败');
      }
    }
  };

  const handleDelete = (id: string) => {
    Modal.confirm({
      title: '确认删除',
      content: '删除后关联此别名的用户将无法使用该模型，确定继续？',
      okText: '确定', cancelText: '取消',
      onOk: async () => {
        try {
          await deleteAlias(id);
          message.success('别名已删除');
          loadAliases();
        } catch { message.error('删除失败'); }
      },
    });
  };

  const columns = [
    { title: '别名', dataIndex: 'alias_name', key: 'alias_name',
      render: (v: string) => <Tag color="blue">{v}</Tag> },
    { title: '指向模型', dataIndex: 'target_model', key: 'target_model',
      render: (v: string) => <Tag>{v}</Tag> },
    { title: '说明', dataIndex: 'description', key: 'description',
      render: (v: string) => v || '-' },
    {
      title: '状态', dataIndex: 'is_active', key: 'is_active',
      render: (v: boolean) => v ? <Tag color="green">启用</Tag> : <Tag color="red">禁用</Tag>,
    },
    {
      title: '操作', key: 'actions',
      render: (_: any, record: any) => (
        <Space>
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
        <h2>模型别名管理</h2>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增别名</Button>
      </div>

      <Table dataSource={aliases} columns={columns} rowKey="id" loading={loading} />

      <Modal
        title={editingAlias ? '编辑别名' : '新增别名'}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
        okText="保存"
        cancelText="取消"
        width={500}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="alias_name" label="别名" rules={[{ required: true }]}>
            <Input placeholder="Jiali_model1" disabled={!!editingAlias} />
          </Form.Item>
          <Form.Item name="target_model" label="指向的真实模型" rules={[{ required: true }]}>
            <Input placeholder="deepseek-v4-flash" />
          </Form.Item>
          <Form.Item name="description" label="说明">
            <Input placeholder="给贾利使用的模型" />
          </Form.Item>
          <Form.Item name="is_active" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
