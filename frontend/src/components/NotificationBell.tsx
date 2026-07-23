import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getUnreadCount, listNotifications, markAllNotificationsRead, markNotificationRead } from '../api/notifications'
import type { Notification, NotificationType } from '../types/models'

function notificationLink(n: Notification): string | null {
  // Mentions in a daily report have no link: GET /daily-reports/:date always
  // resolves to the caller's own report, so there's no page a mentioned user
  // could navigate to that would show the report they were mentioned in.
  if (n.notif_type === 'report_mentioned') return null
  if (n.ticket_id) return `/tickets/${n.ticket_id}`
  if (n.report_date) return `/daily-report/${n.report_date}`
  return null
}

const MESSAGE_KEYS = {
  ticket_assigned: 'notifications.ticketAssigned',
  ticket_overdue: 'notifications.ticketOverdue',
  ticket_commented: 'notifications.ticketCommented',
  report_approved: 'notifications.reportApproved',
  report_rejected: 'notifications.reportRejected',
  ticket_mentioned: 'notifications.ticketMentioned',
  report_mentioned: 'notifications.reportMentioned',
} as const satisfies Record<NotificationType, string>

function NotificationMessage({ n }: { n: Notification }) {
  const { t } = useTranslation()
  return (
    <>
      {t(MESSAGE_KEYS[n.notif_type], {
        actor: n.actor_name ?? '',
        id: n.ticket_id ?? '',
        title: n.ticket_title ?? '',
        date: n.report_date ?? '',
      })}
    </>
  )
}

export function NotificationBell() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)

  const { data: unread } = useQuery({
    queryKey: ['notifications', 'unread-count'],
    queryFn: getUnreadCount,
    refetchInterval: 30_000,
  })

  const { data: page } = useQuery({
    queryKey: ['notifications', 'list'],
    queryFn: () => listNotifications({ per_page: 10 }),
    enabled: open,
  })

  function invalidate() {
    queryClient.invalidateQueries({ queryKey: ['notifications'] })
  }

  const markReadMutation = useMutation({ mutationFn: markNotificationRead, onSuccess: invalidate })
  const markAllMutation = useMutation({ mutationFn: markAllNotificationsRead, onSuccess: invalidate })

  function handleClick(n: Notification) {
    if (!n.read_at) markReadMutation.mutate(n.id)
    setOpen(false)
    const link = notificationLink(n)
    if (link) navigate(link)
  }

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        className="relative rounded p-1.5 text-blue-50 hover:bg-blue-800"
        title={t('notifications.title')}
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
          <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6zM10 18a2 2 0 002-2H8a2 2 0 002 2z" />
        </svg>
        {!!unread?.count && (
          <span className="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-semibold text-white">
            {unread.count > 99 ? '99+' : unread.count}
          </span>
        )}
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute right-0 z-20 mt-2 w-80 rounded border border-gray-200 bg-white text-gray-800 shadow-lg">
            <div className="flex items-center justify-between border-b border-gray-100 px-3 py-2">
              <span className="text-sm font-semibold">{t('notifications.title')}</span>
              <button onClick={() => markAllMutation.mutate()} className="text-xs text-blue-600 hover:underline">
                {t('notifications.markAllRead')}
              </button>
            </div>
            <ul className="max-h-96 overflow-y-auto text-sm">
              {page?.items.length === 0 && (
                <li className="px-3 py-4 text-center text-gray-400">{t('notifications.empty')}</li>
              )}
              {page?.items.map((n) => (
                <li key={n.id}>
                  <button
                    onClick={() => handleClick(n)}
                    className={`block w-full px-3 py-2 text-left hover:bg-gray-50 ${
                      n.read_at ? 'text-gray-500' : 'bg-blue-50 font-medium'
                    }`}
                  >
                    <NotificationMessage n={n} />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </>
      )}
    </div>
  )
}
