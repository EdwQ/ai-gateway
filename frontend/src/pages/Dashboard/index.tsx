import React, { useState, useEffect } from 'react';
import { Row, Col, Card, Statistic, Table, Spin, message, Collapse, Typography } from 'antd';
import { UserOutlined, ThunderboltOutlined, DollarOutlined, TeamOutlined, QuestionCircleOutlined } from '@ant-design/icons';
import ReactECharts from 'echarts-for-react';
import { getDashboard, getDailyStats } from '../../api/client';

const { Paragraph, Text } = Typography;

const apiDocContent = (
  <div>
    <Paragraph>
      <Text strong>📍 服务端口：</Text>
      <code>3000</code>
      {' '}（前端 nginx 代理端口，后端服务运行在 2887）
    </Paragraph>
    <Paragraph>
      <Text strong>🔗 Base URL：</Text>
      <code>http://你的域名:3000/v1/</code>
      {' '}所有 API 请求以此为基础路径
    </Paragraph>
    <Paragraph>
      <Text strong>🔑 认证方式：</Text> 在请求头中携带
      <code>Authorization: Bearer sk-你的token</code>
      {' '}（在"API Token"页面创建并获取你的 Token）
    </Paragraph>
    <Paragraph>
      <Text strong>🤖 模型名称：</Text> 填写系统配置的模型别名（如
      <code>gpt-4</code>
      、<code>qwen-plus</code>
      、<code>claude-3-5-sonnet</code>
      等），可在"模型别名"页面查看完整可用列表
    </Paragraph>
    <Paragraph>
      <Text strong>📝 请求格式：</Text> 与 OpenAI API 完全兼容，支持
      <code>chat/completions</code>
      、<code>embeddings</code>
      、<code>models</code>
      等标准接口
    </Paragraph>
    <Collapse ghost>
      <Collapse.Panel header="Python 示例 (openai SDK)" key="python">
        <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 6, overflow: 'auto' }}>
{`from openai import OpenAI

# 初始化客户端，指定 Base URL 和 API Key
client = OpenAI(
    api_key="sk-你的token",
    base_url="http://你的域名:3000/v1/"
)

# 调用聊天补全接口
response = client.chat.completions.create(
    model="gpt-4",  # 填写模型别名
    messages=[
        {"role": "system", "content": "你是一个有帮助的助手"},
        {"role": "user", "content": "你好，请介绍一下自己"}
    ],
    temperature=0.7
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
    "messages": [
      {"role": "system", "content": "你是一个有帮助的助手"},
      {"role": "user", "content": "你好"}
    ],
    "temperature": 0.7
  }'`}
        </pre>
      </Collapse.Panel>
      <Collapse.Panel header="JavaScript / TypeScript 示例" key="js">
        <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 6, overflow: 'auto' }}>
{`import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'sk-你的token',
  baseURL: 'http://你的域名:3000/v1/'
});

const response = await client.chat.completions.create({
  model: 'gpt-4',
  messages: [
    { role: 'user', content: '你好' }
  ]
});

console.log(response.choices[0].message.content);`}
        </pre>
      </Collapse.Panel>
      <Collapse.Panel header="支持的路由" key="routes">
        <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 6, overflow: 'auto' }}>
{`POST /v1/chat/completions   - 聊天补全（对话生成）
POST /v1/embeddings         - 文本 Embedding（向量化）
GET  /v1/models             - 查询可用模型列表`}
        </pre>
      </Collapse.Panel>
    </Collapse>
  </div>
);

export default function Dashboard() {
  const [loading, setLoading] = useState(true);
  const [stats, setStats] = useState<any>(null);
  const [dailyStats, setDailyStats] = useState<any[]>([]);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      const [dashRes, dailyRes] = await Promise.all([
        getDashboard(),
        getDailyStats(30),
      ]);
      setStats(dashRes.data);
      setDailyStats(dailyRes.data.items || []);
    } catch (err: any) {
      message.error('加载仪表盘数据失败');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return <Spin size="large" style={{ display: 'block', margin: '100px auto' }} />;
  }

  const modelColumns = [
    { title: '模型', dataIndex: 'model', key: 'model' },
    { title: '调用次数', dataIndex: 'calls', key: 'calls', sorter: (a: any, b: any) => a.calls - b.calls },
    { title: '总 Token', dataIndex: 'total_tokens', key: 'total_tokens', sorter: (a: any, b: any) => a.total_tokens - b.total_tokens },
    { title: '费用 (¥)', dataIndex: 'cost', key: 'cost', render: (v: number) => `¥${v.toFixed(2)}`, sorter: (a: any, b: any) => a.cost - b.cost },
  ];

  const chartOption = {
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: dailyStats.map((d: any) => d.date.slice(5)) },
    yAxis: { type: 'value', name: 'Token' },
    series: [{
      name: 'Token 消耗',
      type: 'line',
      smooth: true,
      data: dailyStats.map((d: any) => d.total_tokens),
      areaStyle: { opacity: 0.3 },
    }],
    grid: { left: 60, right: 20, top: 20, bottom: 30 },
  };

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

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="用户总数" value={stats?.total_users || 0} prefix={<UserOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="活跃用户" value={stats?.active_users || 0} prefix={<TeamOutlined />} valueStyle={{ color: '#1677ff' }} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="Token 总量" value={stats?.total_tokens || 0} prefix={<ThunderboltOutlined />} valueStyle={{ color: '#52c41a' }} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="总费用" value={stats?.total_cost || 0} prefix={<DollarOutlined />} precision={2} suffix="¥" valueStyle={{ color: '#faad14' }} />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={14}>
          <Card title="近30天 Token 消耗趋势">
            <ReactECharts option={chartOption} style={{ height: 300 }} />
          </Card>
        </Col>
        <Col xs={24} lg={10}>
          <Card title="模型调用排行">
            <Table
              dataSource={stats?.model_rank || []}
              columns={modelColumns}
              rowKey="model"
              pagination={false}
              size="small"
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
}
