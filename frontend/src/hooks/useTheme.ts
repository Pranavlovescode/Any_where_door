import { useEffect, useState } from 'react'
import { THEME_STORAGE_KEY } from '../constants/storage'
import type { AppTheme } from '../types/models'

const getInitialTheme = (): AppTheme => {
  const stored = localStorage.getItem(THEME_STORAGE_KEY)
  if (stored === 'dark' || stored === 'light') {
    return stored
  }

  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export const useTheme = () => {
  const [theme, setTheme] = useState<AppTheme>(getInitialTheme)

  useEffect(() => {
    const root = document.documentElement
    root.classList.remove('dark', 'light')
    root.classList.add(theme)
    root.setAttribute('data-theme', theme)
    localStorage.setItem(THEME_STORAGE_KEY, theme)
  }, [theme])

  const toggleTheme = () => {
    setTheme((current) => (current === 'dark' ? 'light' : 'dark'))
  }

  return { theme, toggleTheme }
}
