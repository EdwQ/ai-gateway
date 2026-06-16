import React, { useState, useEffect } from 'react';
import { Table, Button, Modal, Input, message, Space, Tag, Tooltip, Typography, Collapse, Card } from 'antd';
import { PlusOutlined, CopyOutlined, ReloadOutlined, DeleteOutlined, KeyOutlined, QuestionCircleOutlined } from '@ant-design/icons';
import { getTokens, createToken, deleteToken, rotateToken } from '../../api/client';

const { Text, Paragraph } = Typography;

const apiDocContent = (
  <div>
    <Paragraph>
      <Text strong>1. Base URL：</Text>
      <code>http://你的域名:3000/v1/</code>
      {' '}（如果通过 nginx 代理，端口为 3000；直接访问后端则为 2887）
    </Paragraph>
    <Paragraph>
      <Text strong>2. 认证方式：</Text> 在请求头中携带
      <code>Authorization: Bearer sk-你的token</code>
      {' '}（将 sk-xxx 替换为你创建的 Token）
    </Paragraph>
    <Paragraph>
      <Text strong>3. 模型名称：</Text> 使用系统配置的模型别名（如
      <code>gpt-4</code>
      、<code>qwen-plus</code>
      等），可在"模型别名"页面查看可用列表
    </Paragraph>
    <Paragraph>
      <Text strong>4. 兼容 OpenAI 格式：</Text> 所有接口与 OpenAI API 完全兼容，可直接替换 OpenAI 的 SDK 配置
    </Paragraph>
    <Collapse ghost>
      <Collapse.Panel header="Python 示例 (openai SDK)" key="python">
        <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 6, overflow: 'auto' }}>
{`from openai import OpenAI

client = OpenAI(
    api_key="sk-你的token",
    base_url="http://你的域名:3000/v1/"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "你好"}]
)
print(response.choices[0].message.content)`}
        </pre>
      </Collapse.Panel>
      <Collapse.Panel header="cURL 示例" key="curl">
        <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 6, overflow: 'auto' }}>
{`curl -X POST http://你的域名:3000/v1/chat/completions \\
  -H "Authorization: Bearer sk-你的token" \\
  -H "Content-Type: application/json" \\
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "你好"}]
  }'`}
        </pre>
      </Collapse.Panel>
      <Collapse.Panel header="支持的路由" key="routes">
        <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 6, overflow: 'auto' }}>
{`POST /v1/chat/completions   - 聊天补全
POST /v1/embeddings         - 文本 Embedding
GET  /v1/models             - 查询可用模型列表`}
        </pre>
      </Collapse.Panel>
    </Collapse>
  </div>
);

export default function Tokens() {
  const [tokens, setTokens] = useState([]);
  const [loading, setLoading] = useState(true);
  const [createModal, setCreateModal] = useState(false);
  const [newTokenName, setNewTokenName] = useState('');
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [createdTokenId, setCreatedTokenId] = useState<string | null>(null);

  const loadTokens = async () => {
    setLoading(true);
    try {
      const res = await getTokens();
      setTokens(res.data.items || []);
    } catch {
      message.error('加载 Token 列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadTokens(); }, []);

  const handleCreate = async () => {
    try {
      const res = await createToken(newTokenName);
      setCreatedToken(res.data.token);
      setCreatedTokenId(res.data.id);
      setCreateModal(false);
      await loadTokens();
    } catch (err: any) {
      message.error(err.response?.data?.detail || '创建失败');
    }
  };

  const handleDelete = (id: string) => {
    Modal.confirm({
      title: '确认删除',
      content: '删除后该 Token 将立即失效，确定继续？',
      okText: '确定',
      cancelText: '取消',
      onOk: async () => {
        try {
          await deleteToken(id);
          message.success('Token 已删除');
          loadTokens();
        } catch {
          message.error('删除失败');
        }
      },
    });
  };

  const handleRotate = (id: string) => {
    Modal.confirm({
      title: '确认轮换',
      content: '轮换后将生成新 Token，旧 Token 立即失效。确定继续？',
      okText: '确定',
      cancelText: '取消',
      onOk: async () => {
        try {
          const res = await rotateToken(id);
          setCreatedToken(res.data.token);
          setCreatedTokenId(res.data.id);
          await loadTokens();
        } catch {
          message.error('轮换失败');
        }
      },
    });
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    message.success('已复制到剪贴板');
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name', render: (v: string) => v || '-' },
    { title: 'Token 前缀', dataIndex: 'token_prefix', key: 'token_prefix', render: (v: string) => <Text code>{v}...</Text> },
    {
      title: '状态', dataIndex: 'is_active', key: 'is_active',
      render: (v: boolean) => v ? <Tag color="green">有效</Tag> : <Tag color="red">已失效</Tag>,
    },
    { title: '创建时间', dataIndex: 'created_at', key: 'created_at', render: (v: string) => v ? new Date(v).toLocaleString() : '-' },
    { title: '最后使用', dataIndex: 'last_used_at', key: 'last_used_at', render: (v: string) => v ? new Date(v).toLocaleString() : '从未使用' },
    {
      title: '操作', key: 'actions',
      render: (_: any, record: any) => (
        <Space>
          <Tooltip title="轮换">
            <Button type="link" icon={<ReloadOutlined />} onClick={() => handleRotate(record.id)} />
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
      <Card
        style={{ marginBottom: 16 }}
        type="inner"
        title={
          <span>
            <QuestionCircleOutlined style={{ marginRight: 8, color: '#1677ff' }} />
            API 使用说明
          </span>
        }
      >
        {apiDocContent}
      </Card>

      <div style={{ marginBottom: 16, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2>API Token 管理</h2>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => { setNewTokenName(''); setCreateModal(true); }}>
          创建 Token
        </Button>
      </div>

      <Table dataSource={tokens} columns={columns} rowKey="id" loading={loading} />

      {/* Create Token Modal */}
      <Modal
        title="创建新 Token"
        open={createModal}
        onOk={handleCreate}
        onCancel={() => setCreateModal(false)}
        okText="创建"
        cancelText="取消"
      >
        <Input
          placeholder="Token 名称（可选）"
          value={newTokenName}
          onChange={(e) => setNewTokenName(e.target.value)}
        />
      </Modal>

      {/* Show Created Token Modal */}
      <Modal
        title="Token 创建成功"
        open={!!createdToken}
        onCancel={() => { setCreatedToken(null); setCreatedTokenId(null); }}
        footer={
          <Button type="primary" onClick={() => { copyToClipboard(createdToken!); }}>
            <CopyOutlined /> 复制 Token
          </Button>
        }
      >
        <div style={{ textAlign: 'center', padding: '20px 0' }}>
          <KeyOutlined style={{ fontSize: 48, color: '#52c41a', marginBottom: 16 }} />
          <p><Text strong style={{ fontSize: 16 }}>请立即复制 Token，关闭后将不再显示！</Text></p>
          <Input.TextArea
            value={createdToken || ''}
            readOnly
            rows={2}
            style={{ fontSize: 14, fontFamily: 'monospace', marginTop: 8 }}
          />
        </div>
      </Modal>
    </div>
  );
}
