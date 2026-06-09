import axios from 'axios';

const api = axios.create({
  baseURL: '',
  timeout: 30000,
});

// Request interceptor: add auth token
api.interceptors.request.use(
  (config) => {
    const token = localStorage.getItem('access_token');
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error) => Promise.reject(error)
);

// Response interceptor: handle 401
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    if (error.response?.status === 401) {
      const refreshToken = localStorage.getItem('refresh_token');
      if (refreshToken && !error.config._retry) {
        error.config._retry = true;
        try {
          const res = await axios.post('/api/v1/auth/refresh', {
            refresh_token: refreshToken,
          });
          localStorage.setItem('access_token', res.data.access_token);
          error.config.headers.Authorization = `Bearer ${res.data.access_token}`;
          return api(error.config);
        } catch {
          localStorage.removeItem('access_token');
          localStorage.removeItem('refresh_token');
          window.location.href = '/login';
        }
      } else {
        localStorage.removeItem('access_token');
        localStorage.removeItem('refresh_token');
        window.location.href = '/login';
      }
    }
    return Promise.reject(error);
  }
);

// Auth APIs
export const login = (authCode: string) =>
  api.post('/api/v1/auth/dingtalk/callback', { auth_code: authCode });

export const devLogin = () =>
  api.post('/api/v1/auth/dev/login');

export const refreshToken = (token: string) =>
  api.post('/api/v1/auth/refresh', { refresh_token: token });

export const getMe = () => api.get('/api/v1/auth/me');

export const logout = () => api.post('/api/v1/auth/logout');

// Token APIs
export const getTokens = () => api.get('/api/v1/tokens');

export const createToken = (name: string) =>
  api.post('/api/v1/tokens', { name });

export const deleteToken = (id: string) =>
  api.delete(`/api/v1/tokens/${id}`);

export const rotateToken = (id: string) =>
  api.post(`/api/v1/tokens/${id}/rotate`);

// User APIs
export const getUsers = (params: Record<string, unknown>) =>
  api.get('/api/v1/users', { params });

export const updateUser = (id: string, data: Record<string, unknown>) =>
  api.patch(`/api/v1/users/${id}`, data);

// Provider APIs
export const getProviders = () => api.get('/api/v1/admin/providers');

export const createProvider = (data: Record<string, unknown>) =>
  api.post('/api/v1/admin/providers', data);

export const updateProvider = (id: string, data: Record<string, unknown>) =>
  api.put(`/api/v1/admin/providers/${id}`, data);

export const deleteProvider = (id: string) =>
  api.delete(`/api/v1/admin/providers/${id}`);

export const checkProviderHealth = (id: string) =>
  api.post(`/api/v1/admin/providers/${id}/check`);

export const discoverModels = (baseUrl: string, apiKey: string) =>
  api.post('/api/v1/admin/providers/discover-models', { base_url: baseUrl, api_key: apiKey });

// Stats APIs
export const getDashboard = () => api.get('/api/v1/stats/dashboard');

export const getDailyStats = (days: number) =>
  api.get('/api/v1/stats/daily', { params: { days } });

export const getMonthlyStats = (months: number) =>
  api.get('/api/v1/stats/monthly', { params: { months } });

export const exportStats = (month: string) =>
  api.get('/api/v1/stats/export', { params: { month }, responseType: 'blob' });

// Audit APIs
export const getAuditLogs = (params: Record<string, unknown>) =>
  api.get('/api/v1/audit/logs', { params });

export default api;
