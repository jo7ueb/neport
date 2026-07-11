import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'

export function AdminHubPage() {
  const { t } = useTranslation()
  const { user } = useAuth()

  if (user?.role !== 'admin') {
    return <p className="text-sm text-gray-500">{t('adminHub.adminOnly')}</p>
  }

  const cards = [
    { to: '/admin/users', title: t('adminUsers.title'), description: t('adminHub.usersDescription') },
    { to: '/admin/workflows', title: t('workflow.title'), description: t('adminHub.workflowsDescription') },
    { to: '/admin/approvers', title: t('adminApprovers.title'), description: t('adminHub.approversDescription') },
    { to: '/admin/settings', title: t('settings.title'), description: t('adminHub.settingsDescription') },
    { to: '/admin/audit-log', title: t('adminAuditLog.title'), description: t('adminHub.auditLogDescription') },
    { to: '/admin/backup', title: t('adminBackup.title'), description: t('adminHub.backupDescription') },
  ]

  return (
    <div>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('adminHub.title')}</h1>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {cards.map((card) => (
          <Link
            key={card.to}
            to={card.to}
            className="rounded border bg-white p-4 hover:border-blue-400 hover:shadow-sm"
          >
            <h2 className="mb-1 font-semibold text-gray-800">{card.title}</h2>
            <p className="text-sm text-gray-500">{card.description}</p>
          </Link>
        ))}
      </div>
    </div>
  )
}
