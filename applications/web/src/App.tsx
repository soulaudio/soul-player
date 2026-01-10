import { Routes, Route, Navigate } from 'react-router-dom';
import { ThemeProvider } from '@soul-player/shared';
import { AuthProvider, useAuth } from './providers/AuthProvider';
import { WebPlayerCommandsProvider } from './providers/WebPlayerCommandsProvider';
import { LoginPage } from './pages/LoginPage';
import { HomePage } from './pages/HomePage';
import { LibraryPage } from './pages/LibraryPage';
import { SettingsPage } from './pages/SettingsPage';
import { MainLayout } from '@soul-player/shared';

function ProtectedRoutes() {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-background">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <WebPlayerCommandsProvider>
      <MainLayout showKeyboardShortcuts={false}>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </MainLayout>
    </WebPlayerCommandsProvider>
  );
}

function App() {
  return (
    <ThemeProvider defaultTheme="dark">
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/*" element={<ProtectedRoutes />} />
        </Routes>
      </AuthProvider>
    </ThemeProvider>
  );
}

export default App;
