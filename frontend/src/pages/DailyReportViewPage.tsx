import { useQuery } from '@tanstack/react-query'
import { Link, useLocation, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getReport } from '../api/dailyReports'
import { getDay } from '../api/workSessions'
import { MarkdownContent } from '../components/MarkdownContent'

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10)
}

export function DailyReportViewPage() {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const { date: dateParam } = useParams()
  const date = dateParam ?? todayIsoDate()
  const isToday = date === todayIsoDate()
  const location = useLocation()
  const justSubmitted = (location.state as { submitted?: boolean } | null)?.submitted ?? false

  const { data: report, isLoading } = useQuery({
    queryKey: ['daily-report', date],
    queryFn: () => getReport(date),
  })

  const { data: day } = useQuery({
    queryKey: ['work-sessions', 'day', date],
    queryFn: () => getDay(date),
  })

  const formattedDate = new Date(`${date}T00:00:00`).toLocaleDateString(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    weekday: 'short',
  })

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {isToday ? t('dailyReport.title') : t('dailyReport.titleForDate', { date: formattedDate })}
        </h1>
        <div className="flex items-center gap-4">
          <Link to="/work/monthly" className="text-sm text-blue-600 hover:underline">
            {t('workSessionMonthly.backToMonthly')}
          </Link>
          {!report?.locked && (
            <Link to={`/daily-report/${date}/edit`} className="text-sm text-blue-600 hover:underline">
              {t('dailyReport.editButton')}
            </Link>
          )}
        </div>
      </div>

      {justSubmitted && (
        <div className="mb-4 rounded border border-green-300 bg-green-50 px-3 py-2 text-sm text-green-700">
          {t('dailyReport.submitSuccess')}
        </div>
      )}

      {day?.decision && (
        <div
          className={`mb-4 rounded border px-3 py-2 text-sm ${
            day.decision.decision === 'approved'
              ? 'border-green-300 bg-green-50 text-green-800'
              : 'border-red-300 bg-red-50 text-red-800'
          }`}
        >
          <p className="font-medium">
            {t(`workSession.dayStatus.${day.decision.decision}` as const)} —{' '}
            {t('workSessionMonthly.decisionBy', { name: day.decision.approver_name })}
          </p>
          {day.decision.comment && <p className="mt-1">{day.decision.comment}</p>}
        </div>
      )}

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : report ? (
        <div className="space-y-4">
        <div className="rounded border bg-white p-4">
          <h2 className="mb-2 text-lg font-semibold text-gray-800">{t('dailyReport.touchedTicketsTitle')}</h2>
          {report.touched_tickets.length === 0 ? (
            <p className="text-sm text-gray-400">{t('dailyReport.noTouchedTickets')}</p>
          ) : (
            <ul className="space-y-1 text-sm">
              {report.touched_tickets.map((ticket) => (
                <li key={ticket.id}>
                  <Link to={`/tickets/${ticket.id}`} className="text-blue-600 hover:underline">
                    #{ticket.id} {ticket.title}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="rounded border bg-white p-4">
          <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('dailyReport.alkptTitle')}</h2>
          <div className="space-y-4 text-sm">
            <div>
              <p className="mb-1 font-medium text-gray-600">{t('dailyReport.achievement')}</p>
              <MarkdownContent text={report.achievement || t('markdown.emptyPreview')} />
            </div>
            <div>
              <p className="mb-1 font-medium text-gray-600">{t('dailyReport.leftover')}</p>
              <MarkdownContent text={report.leftover || t('markdown.emptyPreview')} />
            </div>
            <div>
              <p className="mb-1 font-medium text-gray-600">{t('dailyReport.keep')}</p>
              <MarkdownContent text={report.keep || t('markdown.emptyPreview')} />
            </div>
            <div>
              <p className="mb-1 font-medium text-gray-600">{t('dailyReport.problem')}</p>
              <MarkdownContent text={report.problem || t('markdown.emptyPreview')} />
            </div>
            <div>
              <p className="mb-1 font-medium text-gray-600">{t('dailyReport.try')}</p>
              <MarkdownContent text={report.try || t('markdown.emptyPreview')} />
            </div>
          </div>
        </div>
        </div>
      ) : (
        <div className="rounded border bg-white p-4 text-center">
          <p className="mb-3 text-sm text-gray-500">{t('dailyReport.noReport')}</p>
          <Link
            to={`/daily-report/${date}/edit`}
            className="inline-block rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
          >
            {t('dailyReport.createButton')}
          </Link>
        </div>
      )}
    </div>
  )
}
