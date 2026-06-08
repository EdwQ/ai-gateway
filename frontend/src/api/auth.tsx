import React, { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { login as loginApi, devLogin as devLoginApi, getMe } from './client';

interface User {
  id: string;
  name: string;
  email?: string;
  avatar?: string;
  role: string;
  department_name?: string;
  quota_balance: number;
  quota_used: number;
}

interface AuthContextType {
  user: User | null;
  loading: boolean;
  isAuthenticated: boolean;
  login: (authCode: string) => Promise<void>;
  devLogin: () => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  const initAuth = useCallback(async () => {
    const token = localStorage.getItem('access_token');
    if (token) {
      try {
        const res = await getMe();
        setUser(res.data.user);
      } catch (error: any) {
        localStorage.removeItem('access_token');
        localStorage.removeItem('refresh_token');
        // 清除过期错误的 URL 参数
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.has('access_token') || urlParams.has('error')) {
          window.history.replaceState({}, '', window.location.pathname);
        }
      }
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    initAuth();
  }, [initAuth]);

  const login = async (authCode: string) => {
    const res = await loginApi(authCode);
    const { access_token, refresh_token, user: userData } = res.data;
    localStorage.setItem('access_token', access_token);
    localStorage.setItem('refresh_token', refresh_token);
    setUser(userData);
  };

  const devLogin = async () => {
    const res = await devLoginApi();
    const { access_token, refresh_token, user: userData } = res.data;
    localStorage.setItem('access_token', access_token);
    localStorage.setItem('refresh_token', refresh_token);
    setUser(userData);
  };

  const logout = () => {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    setUser(null);
  };

  return (
    <AuthContext.Provider
      value={{
        user,
        loading,
        isAuthenticated: !!user,
        login,
        devLogin,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within AuthProvider');
  }
  return context;
}
