import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { listSubordinates } from '../api/approvals'
import { UserAvatar } from '../components/UserAvatar'

export function ApprovalsPage() {
  const { t } = useTranslation()

  const { data: subordinates, isLoading } = useQuery({
    queryKey: ['approvals', 'subordinates'],
    queryFn: listSubordinates,
  })

  return (
    <div>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('approvals.title')}</h1>

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <ul className="divide-y rounded border bg-white">
          {subordinates?.map((u) => (
            <li key={u.id}>
              <Link
                to={`/approvals/${u.id}/monthly`}
                className="flex items-center justify-between gap-3 px-4 py-3 hover:bg-gray-50"
              >
                <div className="flex items-center gap-3">
                  <UserAvatar user={u} size={32} />
                  <div>
                    <div className="font-medium text-gray-800">{u.display_name}</div>
                    <div className="text-xs text-gray-500">{u.email}</div>
                  </div>
                </div>
                {u.pending_count > 0 && (
                  <span className="rounded-full bg-orange-100 px-2.5 py-0.5 text-xs font-medium text-orange-700">
                    {t('approvals.pendingCount', { count: u.pending_count })}
                  </span>
                )}
              </Link>
            </li>
          ))}
          {subordinates?.length === 0 && (
            <li className="px-4 py-6 text-center text-sm text-gray-400">{t('approvals.empty')}</li>
          )}
        </ul>
      )}
    </div>
  )
}
