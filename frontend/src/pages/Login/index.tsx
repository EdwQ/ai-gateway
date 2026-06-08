import React, { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Card, Button, message, Typography, Space, Alert, Divider } from 'antd';
import { QrcodeOutlined, LoadingOutlined } from '@ant-design/icons';
import { useAuth } from '../../api/auth';

const { Title, Text } = Typography;

export default function Login() {
  const [qrCodeUrl, setQrCodeUrl] = useState<string>('');
  const [qrCodeFetching, setQrCodeFetching] = useState(true);
  const [dingtalkEnabled, setDingtalkEnabled] = useState(false);
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  // 检查 URL 中是否有钉钉回调回来的 token（处理扫码后重定向）
  useEffect(() => {
    const accessToken = searchParams.get('access_token');
    const refreshToken = searchParams.get('refresh_token');
    const error = searchParams.get('error');

    if (accessToken && refreshToken) {
      // 从钉钉扫码重定向回来的，保存 token 并跳转
      localStorage.setItem('access_token', accessToken);
      localStorage.setItem('refresh_token', refreshToken);
      // 检测是否在弹出窗口中（弹出窗口扫码登录）
      if (window.opener) {
        // 通知父窗口登录成功
        window.opener.location.href = '/';
        window.close();
      } else {
        // 直接页面跳转，重新加载使 AuthProvider 重新初始化
        window.location.href = '/';
      }
      return;
    }

    if (error) {
      const errorMessage = decodeURIComponent(error);
      // 检查是否是 auth_code 过期错误
      if (errorMessage.includes('不存在的临时授权码') || errorMessage.includes('expired')) {
        message.error('登录超时：授权码已过期，请关闭弹窗后重新点击"打开钉钉扫码登录"按钮再次扫描');
      } else if (errorMessage.includes('无效') || errorMessage.includes('invalid')) {
        message.error('无效的授权码，请重新扫码登录');
      } else {
        message.error('登录失败：' + errorMessage);
      }
      // 清除 URL 中的错误参数
      window.history.replaceState({}, '', '/login');
    }
  }, [searchParams]);

  // 获取钉钉 QR code URL
  useEffect(() => {
    const fetchQrCode = async () => {
      try {
        const response = await fetch('/api/v1/auth/dingtalk/qrcode', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
        });

        if (response.ok) {
          const data = await response.json();
          setQrCodeUrl(data.qr_code_url);
          setDingtalkEnabled(true);
        } else {
          setDingtalkEnabled(false);
        }
      } catch (err) {
        console.error('Failed to fetch QR code URL:', err);
        setDingtalkEnabled(false);
      } finally {
        setQrCodeFetching(false);
      }
    };

    fetchQrCode();
  }, []);

  // 打开钉钉扫码弹窗
  const openDingtalkPopup = () => {
    if (!qrCodeUrl) return;
    const width = 800;
    const height = 700;
    const left = (window.screen.width - width) / 2;
    const top = (window.screen.height - height) / 2;
    // Add timestamp to force refresh and avoid caching
    const timestamp = new Date().getTime();
    const urlWithTimestamp = qrCodeUrl.includes('?') 
      ? `${qrCodeUrl}&t=${timestamp}`
      : `${qrCodeUrl}&t=${timestamp}`;
    
    window.open(
      urlWithTimestamp,
      'dingtalk_login',
      `width=${width},height=${height},left=${left},top=${top},resizable=yes,scrollbars=yes`
    );
  };

  return (
    <div style={{
      height: '100vh',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
    }}>
      <Card
        style={{
          width: 480,
          textAlign: 'center',
          boxShadow: '0 8px 32px rgba(0, 0, 0, 0.12)',
          borderRadius: 12,
        }}
      >
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          {/* Logo */}
          <div style={{
            background: 'linear-gradient(135deg, #667eea, #764ba2)',
            borderRadius: '50%',
            width: 80,
            height: 80,
            margin: '0 auto',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            boxShadow: '0 4px 12px rgba(102, 126, 234, 0.4)',
          }}>
            <QrcodeOutlined style={{ fontSize: 40, color: '#fff' }} />
          </div>
          <Title level={3} style={{ margin: '8px 0 0' }}>AI Gateway</Title>
          <Text type="secondary">企业级 AI 能力平台</Text>

          <Divider />

          {/* 加载中 */}
          {qrCodeFetching && (
            <div style={{ textAlign: 'center', padding: 20 }}>
              <LoadingOutlined style={{ fontSize: 32, color: '#1677ff' }} />
              <div style={{ marginTop: 12, color: '#666' }}>加载中...</div>
            </div>
          )}

           {/* 钉钉扫码登录 */}
          {!qrCodeFetching && (
            <>
              {dingtalkEnabled ? (
                <>
                  <Alert
                    message="钉钉扫码登录"
                    description={
                      <div style={{ textAlign: 'left' }}>
                        <p style={{ margin: 0, marginBottom: 8 }}>点击下方按钮，将在新窗口中打开钉钉登录页面</p>
                        <ol style={{ margin: 0, paddingLeft: 20 }}>
                          <li>使用钉钉扫一扫扫描页面上的二维码</li>
                          <li>点击"确认授权"按钮</li>
                          <li><strong>等待页面自动跳回</strong>（不要手动关闭窗口）</li>
                        </ol>
                        <p style={{ margin: '8px 0 0', color: '#ff4d4f', fontSize: 12 }}>
                          ⚠️ 注意：授权码有效期 5 分钟，如提示"授权码过期"，请关闭弹窗后重新扫码
                        </p>
                      </div>
                    }
                    type="info"
                    showIcon
                    style={{ textAlign: 'left' }}
                  />

                  <Button
                    type="primary"
                    size="large"
                    block
                    icon={<QrcodeOutlined />}
                    onClick={openDingtalkPopup}
                    style={{
                      height: 56,
                      fontSize: 16,
                      marginTop: 16,
                    }}
                  >
                    打开钉钉扫码登录
                  </Button>

                  <Text type="secondary" style={{ fontSize: 13, display: 'block', marginTop: 12 }}>
                    新窗口打开后 → 使用钉钉扫一扫 → 授权后自动跳回
                  </Text>
                </>
              ) : (
                <>
                  <Alert
                    message="系统维护中"
                    description="钉钉登录服务暂时不可用，请稍后再试或联系管理员。"
                    type="warning"
                    showIcon
                  />
                </>
              )}
            </>
          )}
        </Space>
      </Card>
    </div>
  );
}
