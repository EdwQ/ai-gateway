import React, { useState, useEffect } from 'react';
import { Row, Col, Card, Statistic, Table, Spin, message, Tabs, DatePicker } from 'antd';
import { ThunderboltOutlined, DollarOutlined, WarningOutlined, ClockCircleOutlined } from '@ant-design/icons';
import ReactECharts from 'echarts-for-react';
import api from '../../api/client';

const { RangePicker } = DatePicker;

export default function Analysis() {
  const [loading, setLoading] = useState(true);
  const [dashboard, setDashboard] = useState<any>(null);
  const [trends, setTrends] = useState<any[]>([]);
  const [topUsers, setTopUsers] = useState<any[]>([]);
  const [topModels, setTopModels] = useState<any[]>([]);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [dashRes, trendRes, usersRes, modelsRes] = await Promise.all([
        api.get('/api/v1/analysis/dashboard'),
        api.get('/api/v1/analysis/trends', { params: { days: 30 } }),
        api.get('/api/v1/analysis/top-users', { params: { days: 30 } }),
        api.get('/api/v1/analysis/top-models', { params: { days: 30 } }),
      ]);
      setDashboard(dashRes.data);
      setTrends(trendRes.data.items || []);
      setTopUsers(usersRes.data.items || []);
      setTopModels(modelsRes.data.items || []);
    } catch (err: any) {
      message.error('加载分析数据失败');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return <Spin size="large" style={{ display: 'block', margin: '100px auto' }} />;
  }

  const trendChartOption = {
    tooltip: { trigger: 'axis' },
    legend: { data: ['调用量', 'Token 消耗', '费用'] },
    xAxis: { type: 'category', data: trends.map((d: any) => d.date.slice(5)) },
    yAxis: [
      { type: 'value', name: '调用量' },
      { type: 'value', name: '费用 (¥)' },
    ],
    series: [
      {
        name: '调用量',
        type: 'bar',
        data: trends.map((d: any) => d.calls),
      },
      {
        name: 'Token 消耗',
        type: 'line',
        smooth: true,
        data: trends.map((d: any) => d.input_tokens + d.output_tokens),
        yAxisIndex: 0,
      },
      {
        name: '费用',
        type: 'line',
        smooth: true,
        data: trends.map((d: any) => d.cost),
        yAxisIndex: 1,
      },
    ],
    grid: { left: 60, right: 60, top: 40, bottom: 30 },
  };

  const userColumns = [
    { title: '排名', key: 'rank', render: (_: any, __: any, i: number) => i + 1, width: 60 },
    { title: '用户名', dataIndex: 'user_name', key: 'user_name' },
    { title: '调用次数', dataIndex: 'calls', key: 'calls', sorter: (a: any, b: any) => a.calls - b.calls },
    { title: '总 Token', dataIndex: 'total_tokens', key: 'total_tokens' },
    { title: '费用 (¥)', dataIndex: 'cost', key: 'cost', render: (v: number) => `¥${v.toFixed(2)}`, sorter: (a: any, b: any) => a.cost - b.cost },
  ];

  const modelColumns = [
    { title: '模型', dataIndex: 'model', key: 'model' },
    { title: '调用次数', dataIndex: 'calls', key: 'calls', sorter: (a: any, b: any) => a.calls - b.calls },
    { title: '总 Token', dataIndex: 'total_tokens', key: 'total_tokens' },
    { title: '费用 (¥)', dataIndex: 'cost', key: 'cost', render: (v: number) => `¥${v.toFixed(2)}`, sorter: (a: any, b: any) => a.cost - b.cost },
    { title: '平均延迟', dataIndex: 'avg_latency_ms', key: 'avg_latency_ms', render: (v: number) => `${v.toFixed(0)}ms` },
  ];

  return (
    <div>
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={4}>
          <Card><Statistic title="总调用次数" value={dashboard?.total_calls || 0} prefix={<ThunderboltOutlined />} /></Card>
        </Col>
        <Col xs={24} sm={12} lg={4}>
          <Card><Statistic title="输入 Token" value={dashboard?.total_input_tokens || 0} prefix={<ThunderboltOutlined />} valueStyle={{ color: '#1677ff' }} /></Card>
        </Col>
        <Col xs={24} sm={12} lg={4}>
          <Card><Statistic title="输出 Token" value={dashboard?.total_output_tokens || 0} prefix={<ThunderboltOutlined />} valueStyle={{ color: '#52c41a' }} /></Card>
        </Col>
        <Col xs={24} sm={12} lg={4}>
          <Card><Statistic title="总费用" value={dashboard?.total_cost || 0} precision={2} prefix={<DollarOutlined />} suffix="¥" valueStyle={{ color: '#faad14' }} /></Card>
        </Col>
        <Col xs={24} sm={12} lg={4}>
          <Card><Statistic title="平均延迟" value={dashboard?.avg_latency_ms || 0} precision={0} prefix={<ClockCircleOutlined />} suffix="ms" /></Card>
        </Col>
        <Col xs={24} sm={12} lg={4}>
          <Card><Statistic title="错误率" value={dashboard?.error_rate || 0} precision={2} prefix={<WarningOutlined />} suffix="%" valueStyle={{ color: dashboard?.error_rate > 5 ? '#ff4d4f' : '#52c41a' }} /></Card>
        </Col>
      </Row>

      <Card title="30天趋势" style={{ marginTop: 16 }}>
        <ReactECharts option={trendChartOption} style={{ height: 350 }} />
      </Card>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={12}>
          <Card title="用户排行 (Top 10)">
            <Table
              dataSource={topUsers}
              columns={userColumns}
              rowKey={(r: any) => r.user_id}
              pagination={false}
              size="small"
            />
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="模型排行 (Top 10)">
            <Table
              dataSource={topModels}
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
