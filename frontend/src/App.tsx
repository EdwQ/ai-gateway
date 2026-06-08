import React, { useState } from 'react';
import { Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom';
import {
  Layout,
  Menu,
  Button,
  Dropdown,
  Spin,
  message,
  Avatar,
  theme,
} from 'antd';
import {
  DashboardOutlined,
  KeyOutlined,
  CloudServerOutlined,
  TeamOutlined,
  AuditOutlined,
  BarChartOutlined,
  LogoutOutlined,
  UserOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { AuthProvider, useAuth } from './api/auth';
import Dashboard from './pages/Dashboard';
import Login from './pages/Login';
import Tokens from './pages/Tokens';
import Providers from './pages/Providers';
import Users from './pages/Users';
import Audit from './pages/Audit';
import Stats from './pages/Stats';

const { Header, Sider, Content } = Layout;

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, loading } = useAuth();
  if (loading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>
        <Spin size="large" />
      </div>
    );
  }
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}

function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const { token: { colorBgContainer, borderRadiusLG } } = theme.useToken();

  const role = user?.role || 'employee';
  const isAdmin = ['admin', 'super_admin', 'finance'].includes(role);

  const menuItems = [
    { key: '/', icon: <DashboardOutlined />, label: '仪表盘' },
    { key: '/tokens', icon: <KeyOutlined />, label: 'API Token' },
    ...(isAdmin ? [
      { key: '/providers', icon: <CloudServerOutlined />, label: 'Provider 管理' },
      { key: '/users', icon: <TeamOutlined />, label: '用户管理' },
      { key: '/audit', icon: <AuditOutlined />, label: '审计日志' },
      { key: '/stats', icon: <BarChartOutlined />, label: '数据统计' },
    ] : []),
  ];

  const handleLogout = () => {
    logout();
    message.success('已退出登录');
    navigate('/login');
  };

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider trigger={null} collapsible collapsed={collapsed} theme="light"
        style={{ borderRight: '1px solid #f0f0f0' }}>
        <div style={{
          height: 64,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: collapsed ? 14 : 18,
          fontWeight: 'bold',
          borderBottom: '1px solid #f0f0f0',
        }}>
          {collapsed ? 'AI' : 'AI Gateway'}
        </div>
        <Menu
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Layout>
        <Header style={{
          padding: '0 24px',
          background: colorBgContainer,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottom: '1px solid #f0f0f0',
        }}>
          <Button
            type="text"
            icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
            onClick={() => setCollapsed(!collapsed)}
          />
          <Dropdown menu={{
            items: [
              { key: 'profile', label: `${user?.name} (${user?.role})`, disabled: true },
              { type: 'divider' },
              {
                key: 'logout',
                icon: <LogoutOutlined />,
                label: '退出登录',
                onClick: handleLogout,
              },
            ],
          }}>
            <div style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 8 }}>
              <Avatar icon={<UserOutlined />} src={user?.avatar} />
              <span>{user?.name}</span>
            </div>
          </Dropdown>
        </Header>
        <Content style={{ margin: 24, padding: 24, background: colorBgContainer, borderRadius: borderRadiusLG }}>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/tokens" element={<Tokens />} />
            {isAdmin && <Route path="/providers" element={<Providers />} />}
            {isAdmin && <Route path="/users" element={<Users />} />}
            {isAdmin && <Route path="/audit" element={<Audit />} />}
            {isAdmin && <Route path="/stats" element={<Stats />} />}
          </Routes>
        </Content>
      </Layout>
    </Layout>
  );
}

export default function App() {
  return (
    <AuthProvider>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/*" element={
          <ProtectedRoute>
            <AppLayout />
          </ProtectedRoute>
        } />
      </Routes>
    </AuthProvider>
  );
}
