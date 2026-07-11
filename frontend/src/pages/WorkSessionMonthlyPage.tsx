import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getMonthly, type DaySummary } from '../api/workSessions'

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

export function WorkSessionMonthlyPage() {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const now = new Date()
  const [year, setYear] = useState(now.getFullYear())
  const [month, setMonth] = useState(now.getMonth() + 1)

  const { data, isLoading } = useQuery({
    queryKey: ['work-sessions', 'monthly', year, month],
    queryFn: () => getMonthly(year, month),
  })

  const summaryByDate = new Map((data?.days ?? []).map((day) => [day.date, day]))
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
          {t('workSessionMonthly.titleForMonth', { month: monthLabel })}
        </h1>
        <Link to="/" className="text-sm text-blue-600 hover:underline">
          {t('workSessionMonthly.backToDashboard')}
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

      {data?.closure.closed && (
        <div className="mb-4 rounded border border-gray-300 bg-gray-50 px-3 py-2 text-sm text-gray-700">
          {t('workSessionMonthly.monthClosedNotice', { name: data.closure.closed_by_name })}
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
              <th className="px-3 py-2">{t('workSessionMonthly.columnDailyReport')}</th>
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
                    {day && (
                      <>
                        <span className={day.status === 'rejected' ? 'font-medium text-red-700' : day.status === 'approved' ? 'font-medium text-green-700' : ''}>
                          {t(`workSession.dayStatus.${day.status}` as const)}
                        </span>
                        {day.decision && (
                          <div className="text-xs text-gray-500">
                            {t('workSessionMonthly.decisionBy', { name: day.decision.approver_name })}
                            {day.decision.comment && <div>{day.decision.comment}</div>}
                          </div>
                        )}
                      </>
                    )}
                  </td>
                  <td className="px-3 py-2">
                    <Link to={`/daily-report/${dateStr}`} className="text-blue-600 hover:underline">
                      {t('workSessionMonthly.viewReport')}
                    </Link>
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
