import { createContext, useContext, useEffect, ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { useAuthStore } from '../stores/auth';

interface AuthContextValue {
  user: { id: string; username: string } | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within AuthProvider');
  }
  return context;
}

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const navigate = useNavigate();
  const { user, isAuthenticated, isLoading, setUser, setLoading, logout: storeLogout } = useAuthStore();

  // Check for existing token on mount
  useEffect(() => {
    const checkAuth = async () => {
      if (apiClient.hasTokens()) {
        try {
          // Verify token is still valid by making a request
          // For now, just assume it's valid and extract user from token
          // In production, you'd want to call a /me endpoint
          const token = apiClient.getAccessToken();
          if (token) {
            // Decode JWT to get user info (basic decode, not verification)
            const payload = JSON.parse(atob(token.split('.')[1]));
            setUser({ id: payload.sub, username: payload.sub });
          } else {
            setUser(null);
          }
        } catch {
          apiClient.clearTokens();
          setUser(null);
        }
      } else {
        setUser(null);
      }
    };

    checkAuth();

    // Handle unauthorized responses
    apiClient.setOnUnauthorized(() => {
      storeLogout();
      navigate('/login');
    });
  }, [setUser, storeLogout, navigate]);

  const login = async (username: string, password: string) => {
    setLoading(true);
    try {
      await apiClient.login(username, password);
      setUser({ id: username, username });
    } finally {
      setLoading(false);
    }
  };

  const logout = () => {
    apiClient.clearTokens();
    storeLogout();
    navigate('/login');
  };

  return (
    <AuthContext.Provider value={{ user, isAuthenticated, isLoading, login, logout }}>
      {children}
    </AuthContext.Provider>
  );
}
