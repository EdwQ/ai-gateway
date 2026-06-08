import React, { useState, useEffect } from 'react';
import { Row, Col, Card, Statistic, Table, Spin, message } from 'antd';
import { UserOutlined, ThunderboltOutlined, DollarOutlined, TeamOutlined } from '@ant-design/icons';
import ReactECharts from 'echarts-for-react';
import { getDashboard, getDailyStats } from '../../api/client';

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
