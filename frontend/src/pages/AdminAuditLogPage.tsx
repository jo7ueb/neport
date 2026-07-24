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
  project_unarchived: 'adminAuditLog.action.projectUnarchived',
  project_member_removed: 'adminAuditLog.action.projectMemberRemoved',
  system_settings_updated: 'adminAuditLog.action.systemSettingsUpdated',
  ticket_deleted: 'adminAuditLog.action.ticketDeleted',
  workflow_deleted: 'adminAuditLog.action.workflowDeleted',
  workflow_status_deleted: 'adminAuditLog.action.workflowStatusDeleted',
  workflow_transition_deleted: 'adminAuditLog.action.workflowTransitionDeleted',
  login: 'adminAuditLog.action.login',
  login_failed: 'adminAuditLog.action.loginFailed',
  logout: 'adminAuditLog.action.logout',
  password_changed: 'adminAuditLog.action.passwordChanged',
  backup_created: 'adminAuditLog.action.backupCreated',
  backup_restored: 'adminAuditLog.action.backupRestored',
  user_created: 'adminAuditLog.action.userCreated',
  milestone_created: 'adminAuditLog.action.milestoneCreated',
  milestone_updated: 'adminAuditLog.action.milestoneUpdated',
  milestone_deleted: 'adminAuditLog.action.milestoneDeleted',
  project_created: 'adminAuditLog.action.projectCreated',
  project_updated: 'adminAuditLog.action.projectUpdated',
  project_member_added: 'adminAuditLog.action.projectMemberAdded',
  project_member_role_changed: 'adminAuditLog.action.projectMemberRoleChanged',
  workflow_created: 'adminAuditLog.action.workflowCreated',
  workflow_updated: 'adminAuditLog.action.workflowUpdated',
  workflow_status_created: 'adminAuditLog.action.workflowStatusCreated',
  workflow_status_updated: 'adminAuditLog.action.workflowStatusUpdated',
  workflow_transition_created: 'adminAuditLog.action.workflowTransitionCreated',
  work_session_edited: 'adminAuditLog.action.workSessionEdited',
  ticket_created: 'adminAuditLog.action.ticketCreated',
  ticket_status_change: 'adminAuditLog.action.ticketStatusChange',
  ticket_assignee_change: 'adminAuditLog.action.ticketAssigneeChange',
  ticket_title_changed: 'adminAuditLog.action.ticketTitleChanged',
  ticket_description_changed: 'adminAuditLog.action.ticketDescriptionChanged',
  ticket_due_date_changed: 'adminAuditLog.action.ticketDueDateChanged',
  ticket_start_date_changed: 'adminAuditLog.action.ticketStartDateChanged',
  ticket_end_date_changed: 'adminAuditLog.action.ticketEndDateChanged',
  ticket_milestone_changed: 'adminAuditLog.action.ticketMilestoneChanged',
  ticket_parent_changed: 'adminAuditLog.action.ticketParentChanged',
  ticket_field_changed: 'adminAuditLog.action.ticketFieldChanged',
  ticket_commented: 'adminAuditLog.action.ticketCommented',
  dependency_added: 'adminAuditLog.action.dependencyAdded',
  dependency_removed: 'adminAuditLog.action.dependencyRemoved',
  report_approved: 'adminAuditLog.action.reportApproved',
  report_rejected: 'adminAuditLog.action.reportRejected',
  month_closed: 'adminAuditLog.action.monthClosed',
  month_reopened: 'adminAuditLog.action.monthReopened',
  work_session_started: 'adminAuditLog.action.workSessionStarted',
  work_session_ended: 'adminAuditLog.action.workSessionEnded',
  attachment_uploaded: 'adminAuditLog.action.attachmentUploaded',
  approver_assigned: 'adminAuditLog.action.approverAssigned',
} as const satisfies Record<AuditAction, string>

// Groups every action into a badge color so the action column reads as
// scannable color blocks instead of a wall of same-styled text. Grouped by
// domain (ticket/report-attendance/user/project/workflow/system), with
// login_failed pulled out of the auth group since it's the one auth event
// worth visually flagging.
type BadgeColor = 'gray' | 'red' | 'purple' | 'amber' | 'teal' | 'blue' | 'green'

const BADGE_CLASSES: Record<BadgeColor, string> = {
  gray: 'bg-gray-100 text-gray-700',
  red: 'bg-red-100 text-red-700',
  purple: 'bg-purple-100 text-purple-700',
  amber: 'bg-amber-100 text-amber-700',
  teal: 'bg-teal-100 text-teal-700',
  blue: 'bg-blue-100 text-blue-700',
  green: 'bg-green-100 text-green-700',
}

const ACTION_COLOR = {
  user_suspended: 'purple',
  user_resumed: 'purple',
  user_deleted: 'purple',
  user_role_changed: 'purple',
  user_approver_flag_changed: 'purple',
  user_password_reset: 'purple',
  approver_removed: 'purple',
  project_archived: 'amber',
  project_unarchived: 'amber',
  project_member_removed: 'amber',
  system_settings_updated: 'gray',
  ticket_deleted: 'blue',
  workflow_deleted: 'teal',
  workflow_status_deleted: 'teal',
  workflow_transition_deleted: 'teal',
  login: 'gray',
  login_failed: 'red',
  logout: 'gray',
  password_changed: 'gray',
  backup_created: 'gray',
  backup_restored: 'gray',
  user_created: 'purple',
  milestone_created: 'amber',
  milestone_updated: 'amber',
  milestone_deleted: 'amber',
  project_created: 'amber',
  project_updated: 'amber',
  project_member_added: 'amber',
  project_member_role_changed: 'amber',
  workflow_created: 'teal',
  workflow_updated: 'teal',
  workflow_status_created: 'teal',
  workflow_status_updated: 'teal',
  workflow_transition_created: 'teal',
  work_session_edited: 'green',
  ticket_created: 'blue',
  ticket_status_change: 'blue',
  ticket_assignee_change: 'blue',
  ticket_title_changed: 'blue',
  ticket_description_changed: 'blue',
  ticket_due_date_changed: 'blue',
  ticket_start_date_changed: 'blue',
  ticket_end_date_changed: 'blue',
  ticket_milestone_changed: 'blue',
  ticket_parent_changed: 'blue',
  ticket_field_changed: 'blue',
  ticket_commented: 'blue',
  dependency_added: 'blue',
  dependency_removed: 'blue',
  report_approved: 'green',
  report_rejected: 'green',
  month_closed: 'green',
  month_reopened: 'green',
  work_session_started: 'green',
  work_session_ended: 'green',
  attachment_uploaded: 'gray',
  approver_assigned: 'purple',
} as const satisfies Record<AuditAction, BadgeColor>

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
        <table className="w-full table-fixed overflow-hidden rounded border bg-white text-sm">
          <thead className="bg-gray-50 text-left text-xs uppercase text-gray-500">
            <tr>
              <th className="w-40 px-3 py-2">{t('adminAuditLog.columnOccurredAt')}</th>
              <th className="w-32 px-3 py-2">{t('adminAuditLog.columnActor')}</th>
              <th className="w-52 px-3 py-2">{t('adminAuditLog.columnAction')}</th>
              <th className="w-40 px-3 py-2">{t('adminAuditLog.columnTarget')}</th>
              <th className="px-3 py-2">{t('adminAuditLog.columnDetail')}</th>
              <th className="w-32 px-3 py-2">{t('adminAuditLog.columnIp')}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {data?.items.length === 0 && (
              <tr>
                <td colSpan={6} className="px-3 py-6 text-center text-gray-400">
                  {t('adminAuditLog.empty')}
                </td>
              </tr>
            )}
            {data?.items.map((entry) => (
              <tr key={entry.id}>
                <td className="px-3 py-2 whitespace-nowrap text-gray-500">
                  {new Date(entry.occurred_at).toLocaleString(i18n.language)}
                </td>
                <td className="truncate px-3 py-2">{entry.actor_name}</td>
                <td className="px-3 py-2">
                  <span
                    className={`inline-block truncate rounded-full px-2.5 py-0.5 text-xs font-medium ${BADGE_CLASSES[ACTION_COLOR[entry.action]]}`}
                  >
                    {t(ACTION_KEYS[entry.action] ?? entry.action)}
                  </span>
                </td>
                <td className="truncate px-3 py-2">
                  {entry.target_name ?? (entry.target_id !== null ? `${entry.target_type} #${entry.target_id}` : entry.target_type)}
                </td>
                <td className="break-words px-3 py-2 text-gray-500">
                  {entry.from_text && entry.to_text
                    ? `${entry.from_text} → ${entry.to_text}`
                    : entry.to_text ?? entry.from_text ?? ''}
                </td>
                <td className="px-3 py-2 whitespace-nowrap text-gray-500">{entry.ip_address ?? '—'}</td>
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
