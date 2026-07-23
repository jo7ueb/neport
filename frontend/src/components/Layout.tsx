import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import { useSettings } from '../context/SettingsContext'
import { NotificationBell } from './NotificationBell'
import { UserAvatar } from './UserAvatar'

export function Layout() {
  const { t } = useTranslation()
  const { user, logout } = useAuth()
  const { maintenanceMode } = useSettings()
  const navigate = useNavigate()
  const location = useLocation()
  // The WBS/Gantt split view needs every pixel of width it can get (long
  // ticket titles alongside a horizontally-scrolling chart) - the standard
  // max-w-5xl reading-width constraint just wastes a wide monitor there.
  const isWideLayout = /^\/projects\/[^/]+\/wbs$/.test(location.pathname)

  async function handleLogout() {
    await logout()
    navigate('/login')
  }

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `rounded px-3 py-1.5 text-sm ${isActive ? 'bg-white text-blue-700' : 'text-blue-50 hover:bg-blue-800'}`

  return (
    <div className="flex min-h-screen flex-col bg-gray-50">
      <header className="bg-blue-700">
        <div className="mx-auto flex max-w-5xl flex-wrap items-center justify-between gap-2 px-4 py-3">
          <Link to="/" className="flex items-center gap-2 text-lg font-semibold text-white">
            <img src="/neport-favicon.png" alt="" className="h-7 w-7" />
            Neport
          </Link>
          <nav className="flex flex-wrap items-center gap-1">
            <NavLink to="/" className={linkClass} end>
              {t('nav.dashboard')}
            </NavLink>
            <NavLink to="/projects" className={linkClass}>
              {t('nav.projects')}
            </NavLink>
            <NavLink to="/work/monthly" className={linkClass}>
              {t('nav.work')}
            </NavLink>
            <NavLink to="/daily-report" className={linkClass}>
              {t('nav.dailyReport')}
            </NavLink>
            {user?.is_approver && (
              <NavLink to="/approvals" className={linkClass}>
                {t('nav.approvals')}
              </NavLink>
            )}
            {user?.role === 'admin' && (
              <NavLink to="/admin" className={linkClass}>
                {t('nav.admin')}
              </NavLink>
            )}
          </nav>
          <div className="flex items-center gap-4 text-sm text-blue-50">
            <NotificationBell />
            <Link to="/account" className="flex items-center gap-2 hover:underline" title={t('nav.account')}>
              {user && <UserAvatar user={user} size={28} />}
              {user?.display_name}
            </Link>
            <button
              onClick={handleLogout}
              className="rounded border border-blue-300 px-3 py-1 hover:bg-blue-800"
            >
              {t('nav.logout')}
            </button>
          </div>
        </div>
      </header>
      {maintenanceMode && (
        <div className="bg-amber-100 px-4 py-2 text-center text-sm font-medium text-amber-800">
          {t('layout.maintenanceBanner')}
        </div>
      )}
      <main className={`mx-auto w-full flex-1 py-6 ${isWideLayout ? 'max-w-none px-3' : 'max-w-5xl px-4'}`}>
        <Outlet />
      </main>
      <footer className="bg-blue-700">
        <div className="mx-auto max-w-5xl px-4 py-3 text-center text-blue-100">
          <div className="text-base italic text-blue-200">{t('footer.slogan')}</div>
          <div className="mt-0.5 text-xs">
            {t('footer.version', { version: import.meta.env.VITE_GIT_COMMIT || 'dev' })}
            {import.meta.env.VITE_GIT_DIRTY === '1' && (
              <span className="ml-1 font-semibold text-amber-300">{t('footer.dirty')}</span>
            )}
          </div>
        </div>
      </footer>
    </div>
  )
}
