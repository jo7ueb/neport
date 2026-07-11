import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { closeMonth, getSubordinateMonthly, listSubordinates, reopenMonth } from '../api/approvals'
import type { DaySummary } from '../api/workSessions'
import { getErrorMessage } from '../i18n/apiErrors'

function computeWorkedMinutes(day: DaySummary): number {
  let totalMs = 0
  for (const s of day.sessions) {
    if (s.ended_at) {
      totalMs += new Date(s.ended_at).getTime() - new Date(s.started_at).getTime()
    }
  }
  return Math.round(totalMs / 60000)
}

function pad2(n: number): string {
  return n.toString().padStart(2, '0')
}

export function ApprovalMonthlyPage() {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const { userId: userIdParam } = useParams()
  const userId = Number(userIdParam)
  const queryClient = useQueryClient()

  const now = new Date()
  const [year, setYear] = useState(now.getFullYear())
  const [month, setMonth] = useState(now.getMonth() + 1)
  const [error, setError] = useState<string | null>(null)

  const { data: subordinates } = useQuery({ queryKey: ['approvals', 'subordinates'], queryFn: listSubordinates })
  const subject = subordinates?.find((u) => u.id === userId)

  const { data, isLoading } = useQuery({
    queryKey: ['approvals', 'subordinates', userId, 'monthly', year, month],
    queryFn: () => getSubordinateMonthly(userId, year, month),
  })

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ['approvals', 'subordinates', userId, 'monthly', year, month] })

  const closeMutation = useMutation({
    mutationFn: () => closeMonth(userId, year, month),
    onSuccess: () => {
      invalidate()
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'approvals.closeFailed')),
  })
  const reopenMutation = useMutation({
    mutationFn: () => reopenMonth(userId, year, month),
    onSuccess: () => {
      invalidate()
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'approvals.reopenFailed')),
  })

  const summaryByDate = new Map((data?.days ?? []).map((day) => [day.date, day]))
  const pendingCount = (data?.days ?? []).filter((day) => day.status === 'valid').length
  const daysInMonth = new Date(year, month, 0).getDate()
  const monthLabel = new Date(year, month - 1, 1).toLocaleDateString(locale, { month: 'long' })

  function formatMinutes(totalMinutes: number): string {
    const h = Math.floor(totalMinutes / 60)
    const m = totalMinutes % 60
    return t('workSessionMonthly.hoursMinutes', { hours: h, minutes: m })
  }

  function formatTime(iso: string): string {
    return new Date(iso).toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })
  }

  function goToPrevMonth() {
    if (month === 1) {
      setYear((y) => y - 1)
      setMonth(12)
    } else {
      setMonth((m) => m - 1)
    }
  }

  function goToNextMonth() {
    if (month === 12) {
      setYear((y) => y + 1)
      setMonth(1)
    } else {
      setMonth((m) => m + 1)
    }
  }

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {subject
            ? t('approvals.monthlyTitleFor', { name: subject.display_name, month: monthLabel })
            : t('approvals.monthlyTitle', { month: monthLabel })}
        </h1>
        <Link to="/approvals" className="text-sm text-blue-600 hover:underline">
          {t('approvals.backToList')}
        </Link>
      </div>

      <div className="mb-4 flex items-center justify-center gap-4 text-sm">
        <button
          onClick={goToPrevMonth}
          className="rounded border border-gray-300 px-3 py-1 hover:bg-gray-50"
          aria-label={t('workSessionMonthly.prevMonth')}
        >
          ‹
        </button>
        <span className="min-w-[8rem] text-center text-base font-semibold text-gray-800">
          {new Date(year, month - 1, 1).toLocaleDateString(locale, { year: 'numeric', month: 'long' })}
        </span>
        <button
          onClick={goToNextMonth}
          className="rounded border border-gray-300 px-3 py-1 hover:bg-gray-50"
          aria-label={t('workSessionMonthly.nextMonth')}
        >
          ›
        </button>
      </div>

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      {data && (
        <div className="mb-4 flex items-center justify-between rounded border bg-white p-3 text-sm">
          <div>
            {data.closure.closed ? (
              <span className="text-gray-700">
                {t('approvals.closedNotice', { name: data.closure.closed_by_name })}
              </span>
            ) : (
              <span className="text-gray-500">{t('approvals.openNotice')}</span>
            )}
            {!data.closure.closed && pendingCount > 0 && (
              <div className="mt-0.5 text-xs font-medium text-orange-700">
                {t('approvals.pendingCount', { count: pendingCount })}
              </div>
            )}
          </div>
          {data.closure.closed ? (
            <button
              onClick={() => reopenMutation.mutate()}
              disabled={reopenMutation.isPending}
              className="rounded border border-orange-400 px-3 py-1 text-orange-600 hover:bg-orange-50 disabled:opacity-50"
            >
              {t('approvals.reopenMonth')}
            </button>
          ) : (
            <button
              onClick={() => closeMutation.mutate()}
              disabled={closeMutation.isPending}
              className="rounded bg-blue-600 px-3 py-1 text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('approvals.closeMonth')}
            </button>
          )}
        </div>
      )}

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <table className="w-full overflow-hidden rounded border bg-white text-sm">
          <thead className="border-b bg-gray-50 text-left text-gray-500">
            <tr>
              <th className="px-3 py-2">{t('workSessionMonthly.columnDate')}</th>
              <th className="px-3 py-2">{t('workSessionMonthly.columnWeekday')}</th>
              <th className="px-3 py-2">{t('workSessionMonthly.columnSessions')}</th>
              <th className="px-3 py-2">{t('workSessionMonthly.columnWorkedTime')}</th>
              <th className="px-3 py-2">{t('workSessionMonthly.columnStatus')}</th>
              <th className="px-3 py-2">{t('approvals.columnDecide')}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {Array.from({ length: daysInMonth }, (_, i) => i + 1).map((d) => {
              const date = new Date(year, month - 1, d)
              const dateStr = `${year}-${pad2(month)}-${pad2(d)}`
              const dayOfWeek = date.getDay()
              const weekdayLabel = date.toLocaleDateString(locale, { weekday: 'short' })
              const day = summaryByDate.get(dateStr)
              const isSunday = dayOfWeek === 0
              const isSaturday = dayOfWeek === 6

              const weekdayClass = isSunday
                ? 'text-red-600 font-medium'
                : isSaturday
                  ? 'text-blue-600 font-medium'
                  : 'text-gray-700'
              const rowClass = isSunday ? 'bg-red-50' : isSaturday ? 'bg-blue-50' : ''

              return (
                <tr key={dateStr} className={rowClass}>
                  <td className="px-3 py-2">{d}</td>
                  <td className={`px-3 py-2 ${weekdayClass}`}>{weekdayLabel}</td>
                  <td className="px-3 py-2">
                    {day && day.sessions.length > 0 ? (
                      <ul className="space-y-0.5">
                        {day.sessions.map((s) => (
                          <li key={s.id}>
                            {formatTime(s.started_at)} – {s.ended_at ? formatTime(s.ended_at) : t('workSessionMonthly.ongoing')}
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <span className="text-gray-400">{t('workSessionMonthly.noSessions')}</span>
                    )}
                  </td>
                  <td className="px-3 py-2">{day ? formatMinutes(computeWorkedMinutes(day)) : ''}</td>
                  <td className="px-3 py-2">
                    {day ? t(`workSession.dayStatus.${day.status}` as const) : ''}
                  </td>
                  <td className="px-3 py-2">
                    {day && (
                      <Link to={`/approvals/${userId}/${dateStr}`} className="text-blue-600 hover:underline">
                        {day.status === 'valid' ? t('approvals.decideAction') : t('approvals.viewAction')}
                      </Link>
                    )}
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      )}
    </div>
  )
}
