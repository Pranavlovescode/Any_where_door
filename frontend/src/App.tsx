import { Navigate, Route, Routes } from 'react-router-dom'
import { useTheme } from './hooks/useTheme'
import { AppLayout } from './layout/AppLayout'
import { DashboardPage } from './pages/DashboardPage'
import { LoginPage } from './pages/LoginPage'

const App = () => {
  const { theme, toggleTheme } = useTheme()

  return (
    <Routes>
      <Route element={<AppLayout theme={theme} onToggleTheme={toggleTheme} />}>
        <Route index element={<Navigate to="/login" replace />} />
        <Route path="login" element={<LoginPage />} />
        <Route path="dashboard" element={<DashboardPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/login" replace />} />
    </Routes>
  )
}

export default App
