import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import { listAuditLog } from '../api/auditLog'
import { listAllUsers } from '../api/users'
import { Pagination } from '../components/Pagination'
import type { AuditAction } from '../types/models'

const ACTION_KEYS = {
  user_suspended: 'adminAuditLog.action.userSuspended',
  user_resumed: 'adminAuditLog.action.userResumed',
  user_deleted: 'adminAuditLog.action.userDeleted',
  user_role_changed: 'adminAuditLog.action.userRoleChanged',
  user_approver_flag_changed: 'adminAuditLog.action.userApproverFlagChanged',
  user_password_reset: 'adminAuditLog.action.userPasswordReset',
  approver_removed: 'adminAuditLog.action.approverRemoved',
  project_archived: 'adminAuditLog.action.projectArchived',
  project_member_removed: 'adminAuditLog.action.projectMemberRemoved',
  system_settings_updated: 'adminAuditLog.action.systemSettingsUpdated',
  ticket_deleted: 'adminAuditLog.action.ticketDeleted',
  workflow_deleted: 'adminAuditLog.action.workflowDeleted',
  workflow_status_deleted: 'adminAuditLog.action.workflowStatusDeleted',
  workflow_transition_deleted: 'adminAuditLog.action.workflowTransitionDeleted',
} as const satisfies Record<AuditAction, string>

export function AdminAuditLogPage() {
  const { t, i18n } = useTranslation()
  const { user: currentUser } = useAuth()
  const [userId, setUserId] = useState<string>('')
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useState(25)

  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })

  const { data, isLoading } = useQuery({
    queryKey: ['audit-log', userId, from, to, page, perPage],
    queryFn: () =>
      listAuditLog({
        user_id: userId ? Number(userId) : undefined,
        from: from || undefined,
        to: to || undefined,
        page,
        per_page: perPage,
      }),
  })

  function handleFilterChange(setter: (v: string) => void) {
    return (value: string) => {
      setter(value)
      setPage(1)
    }
  }

  if (currentUser?.role !== 'admin') {
    return (
      <div className="rounded border bg-white p-6 text-center text-gray-500">
        {t('adminAuditLog.adminOnly')}
      </div>
    )
  }

  return (
    <div>
      <Link to="/admin" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('adminHub.backToHub')}
      </Link>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('adminAuditLog.title')}</h1>

      <div className="mb-4 flex flex-wrap items-end gap-3 rounded border bg-white p-3 text-sm">
        <label className="block">
          <span className="mb-1 block text-gray-600">{t('adminAuditLog.filterUser')}</span>
          <select
            value={userId}
            onChange={(e) => handleFilterChange(setUserId)(e.target.value)}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="">{t('adminAuditLog.filterUserAll')}</option>
            {users?.map((u) => (
              <option key={u.id} value={u.id}>
                {u.display_name}
              </option>
            ))}
          </select>
        </label>
        <label className="block">
          <span className="mb-1 block text-gray-600">{t('adminAuditLog.filterFrom')}</span>
          <input
            type="date"
            value={from}
            max={to || undefined}
            onChange={(e) => handleFilterChange(setFrom)(e.target.value)}
            className="rounded border border-gray-300 px-2 py-1"
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-gray-600">{t('adminAuditLog.filterTo')}</span>
          <input
            type="date"
            value={to}
            min={from || undefined}
            onChange={(e) => handleFilterChange(setTo)(e.target.value)}
            className="rounded border border-gray-300 px-2 py-1"
          />
        </label>
      </div>

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <table className="w-full overflow-hidden rounded border bg-white text-sm">
          <thead className="bg-gray-50 text-left text-xs uppercase text-gray-500">
            <tr>
              <th className="px-3 py-2">{t('adminAuditLog.columnOccurredAt')}</th>
              <th className="px-3 py-2">{t('adminAuditLog.columnActor')}</th>
              <th className="px-3 py-2">{t('adminAuditLog.columnAction')}</th>
              <th className="px-3 py-2">{t('adminAuditLog.columnTarget')}</th>
              <th className="px-3 py-2">{t('adminAuditLog.columnDetail')}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {data?.items.length === 0 && (
              <tr>
                <td colSpan={5} className="px-3 py-6 text-center text-gray-400">
                  {t('adminAuditLog.empty')}
                </td>
              </tr>
            )}
            {data?.items.map((entry) => (
              <tr key={entry.id}>
                <td className="px-3 py-2 whitespace-nowrap text-gray-500">
                  {new Date(entry.occurred_at).toLocaleString(i18n.language)}
                </td>
                <td className="px-3 py-2">{entry.actor_name}</td>
                <td className="px-3 py-2">{t(ACTION_KEYS[entry.action] ?? entry.action)}</td>
                <td className="px-3 py-2">
                  {entry.target_name ?? (entry.target_id !== null ? `${entry.target_type} #${entry.target_id}` : entry.target_type)}
                </td>
                <td className="px-3 py-2 text-gray-500">
                  {entry.from_text && entry.to_text
                    ? `${entry.from_text} → ${entry.to_text}`
                    : entry.to_text ?? entry.from_text ?? ''}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {data && (
        <Pagination
          page={page}
          perPage={perPage}
          total={data.total}
          onPageChange={setPage}
          onPerPageChange={(n) => {
            setPerPage(n)
            setPage(1)
          }}
        />
      )}
    </div>
  )
}
