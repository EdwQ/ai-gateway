import React, { useState, useEffect } from 'react';
import { Card, Row, Col, DatePicker, Button, message, Spin } from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import ReactECharts from 'echarts-for-react';
import dayjs from 'dayjs';
import { getDailyStats, getMonthlyStats, exportStats } from '../../api/client';

export default function Stats() {
  const [dailyStats, setDailyStats] = useState<any[]>([]);
  const [monthlyStats, setMonthlyStats] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [exportMonth, setExportMonth] = useState(dayjs().format('YYYY-MM'));

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [dailyRes, monthlyRes] = await Promise.all([
        getDailyStats(30),
        getMonthlyStats(6),
      ]);
      setDailyStats(dailyRes.data.items || []);
      setMonthlyStats(monthlyRes.data.items || []);
    } catch {
      message.error('加载统计数据失败');
    } finally {
      setLoading(false);
    }
  };

  const handleExport = async () => {
    try {
      const res = await exportStats(exportMonth);
      const url = URL.createObjectURL(new Blob([res.data]));
      const a = document.createElement('a');
      a.href = url;
      a.download = `usage_${exportMonth}.csv`;
      a.click();
      URL.revokeObjectURL(url);
      message.success('导出成功');
    } catch {
      message.error('导出失败');
    }
  };

  const dailyChartOption = {
    tooltip: { trigger: 'axis' },
    legend: { data: ['Token 消耗', '费用 (¥)'] },
    xAxis: { type: 'category', data: dailyStats.map((d: any) => d.date.slice(5)) },
    yAxis: [
      { type: 'value', name: 'Token' },
      { type: 'value', name: '费用 (¥)' },
    ],
    series: [
      {
        name: 'Token 消耗', type: 'bar', data: dailyStats.map((d: any) => d.total_tokens),
        itemStyle: { color: '#1677ff' },
      },
      {
        name: '费用 (¥)', type: 'line', yAxisIndex: 1,
        data: dailyStats.map((d: any) => d.total_cost),
        itemStyle: { color: '#52c41a' },
      },
    ],
    grid: { left: 60, right: 60, top: 40, bottom: 30 },
  };

  const monthlyChartOption = {
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: monthlyStats.map((d: any) => d.month) },
    yAxis: { type: 'value', name: 'Token' },
    series: [{
      name: '月度 Token', type: 'bar', data: monthlyStats.map((d: any) => d.total_tokens),
      itemStyle: { color: '#1677ff' },
      barWidth: '40%',
    }],
    grid: { left: 60, right: 20, top: 20, bottom: 30 },
  };

  if (loading) {
    return <Spin size="large" style={{ display: 'block', margin: '100px auto' }} />;
  }

  return (
    <div>
      <h2>数据统计</h2>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={12}>
          <Card title="近30天日统计">
            <ReactECharts option={dailyChartOption} style={{ height: 350 }} />
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="近6月月度统计">
            <ReactECharts option={monthlyChartOption} style={{ height: 350 }} />
          </Card>
        </Col>
      </Row>

      <Card title="报表导出" style={{ marginTop: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <span>选择月份：</span>
          <DatePicker
            picker="month"
            value={dayjs(exportMonth)}
            onChange={(d) => setExportMonth(d?.format('YYYY-MM') || exportMonth)}
          />
          <Button type="primary" icon={<DownloadOutlined />} onClick={handleExport}>
            导出 CSV
          </Button>
        </div>
      </Card>
    </div>
  );
}
