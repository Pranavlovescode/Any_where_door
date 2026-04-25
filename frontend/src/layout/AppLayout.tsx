import { Moon, Sun } from 'lucide-react'
import { AnimatePresence, motion } from 'framer-motion'
import { NavLink, Outlet, useLocation } from 'react-router-dom'
import heroImg from '../assets/hero.png'
import type { AppTheme } from '../types/models'

type AppLayoutProps = {
  theme: AppTheme
  onToggleTheme: () => void
}

const navClass = ({ isActive }: { isActive: boolean }) =>
  `rounded-md border px-3 py-1.5 text-xs uppercase tracking-[0.08em] transition ${
    isActive
      ? 'border-emerald-300/60 bg-emerald-300/20 text-emerald-100'
      : 'border-white/10 bg-white/5 text-white/70 hover:bg-white/10'
  }`

export const AppLayout = ({ theme, onToggleTheme }: AppLayoutProps) => {
  const location = useLocation()
  const currentPath = location.pathname

  return (
    <div className="min-h-screen  text-zinc-100">
      <div className="atmosphere pointer-events-none fixed inset-0 -z-10" />

      <header className="explorer-shell mx-auto mt-4 flex w-full max-w-[1320px] items-center justify-between rounded-xl border border-white/10 bg-black/45 px-4 py-3 backdrop-blur-md md:px-6">
        <div className="flex items-center gap-3">
          <img src={heroImg} alt="Anywhere Door" className="h-7 w-7 rounded object-contain" />
          <div>
            <p className="text-[11px] uppercase tracking-[0.16em] text-zinc-400">Anywhere Door Agent</p>
            <p className="font-display text-sm text-zinc-100">
              {currentPath === '/dashboard' ? 'Virtual File Explorer' : 'Authentication Portal'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <NavLink to="/login" className={navClass}>
            Login
          </NavLink>
          <NavLink to="/dashboard" className={navClass}>
            Dashboard
          </NavLink>
          <button
            type="button"
            onClick={onToggleTheme}
            className="rounded-md border border-white/10 bg-white/5 p-2 text-zinc-200 transition hover:bg-white/10"
            aria-label="Toggle theme"
          >
            {theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
          </button>
        </div>
      </header>

      <main className="mx-auto w-full max-w-[1320px] px-4 pb-8 pt-4 md:px-6">
        <AnimatePresence mode="wait">
          <motion.div
            key={location.pathname}
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.24 }}
          >
            <Outlet />
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  )
}
