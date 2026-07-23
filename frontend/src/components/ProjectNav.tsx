import { NavLink } from 'react-router-dom'
import { useTranslation } from 'react-i18next'

export function ProjectNav({ projectId, archived }: { projectId: number; archived?: boolean }) {
  const { t } = useTranslation()
  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `rounded px-3 py-1.5 text-sm ${isActive ? 'bg-blue-600 text-white' : 'text-gray-600 hover:bg-gray-100'}`

  return (
    <>
      <nav className="mb-4 flex gap-2 rounded border bg-white p-2">
        <NavLink to={`/projects/${projectId}/tickets`} className={linkClass} end>
          {t('projectNav.tickets')}
        </NavLink>
        <NavLink to={`/projects/${projectId}/kanban`} className={linkClass}>
          {t('projectNav.kanban')}
        </NavLink>
        <NavLink to={`/projects/${projectId}/milestones`} className={linkClass}>
          {t('projectNav.milestones')}
        </NavLink>
        <NavLink to={`/projects/${projectId}/wbs`} className={linkClass}>
          {t('projectNav.wbs')}
        </NavLink>
        <NavLink to={`/projects/${projectId}/members`} className={linkClass}>
          {t('projectNav.members')}
        </NavLink>
      </nav>
      {archived && (
        <div className="mb-4 rounded border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-800">
          {t('projectNav.archivedBanner')}
        </div>
      )}
    </>
  )
}
