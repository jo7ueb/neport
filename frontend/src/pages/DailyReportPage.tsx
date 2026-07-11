import { useEffect, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getReport, getTodayTickets, submitReport } from '../api/dailyReports'
import { getErrorMessage } from '../i18n/apiErrors'
import { MarkdownField } from '../components/MarkdownField'

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10)
}

export function DailyReportPage() {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const { date: dateParam } = useParams()
  const date = dateParam ?? todayIsoDate()
  const isToday = date === todayIsoDate()

  const { data: tickets, isLoading: ticketsLoading } = useQuery({
    queryKey: ['daily-report', 'today-tickets'],
    queryFn: getTodayTickets,
    enabled: isToday,
  })

  const { data: existingReport } = useQuery({
    queryKey: ['daily-report', date],
    queryFn: () => getReport(date),
  })

  const [achievement, setAchievement] = useState('')
  const [leftover, setLeftover] = useState('')
  const [keep, setKeep] = useState('')
  const [problem, setProblem] = useState('')
  const [tryField, setTryField] = useState('')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setAchievement(existingReport?.achievement ?? '')
    setLeftover(existingReport?.leftover ?? '')
    setKeep(existingReport?.keep ?? '')
    setProblem(existingReport?.problem ?? '')
    setTryField(existingReport?.try ?? '')
    setError(null)
  }, [date, existingReport])

  const submitMutation = useMutation({
    mutationFn: () =>
      submitReport({
        report_date: date,
        achievement: achievement || undefined,
        leftover: leftover || undefined,
        keep: keep || undefined,
        problem: problem || undefined,
        try: tryField || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['daily-report', 'history'] })
      queryClient.invalidateQueries({ queryKey: ['daily-report', date] })
      queryClient.invalidateQueries({ queryKey: ['work-sessions', 'today'] })
      queryClient.invalidateQueries({ queryKey: ['work-sessions', 'current'] })
      navigate(`/daily-report/${date}`, { replace: true, state: { submitted: true } })
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'dailyReport.submitFailed'))
    },
  })

  function handleSubmit() {
    submitMutation.mutate()
  }

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
        {!isToday && (
          <Link to={`/daily-report/${date}`} className="text-sm text-blue-600 hover:underline">
            {t('dailyReport.backToView')}
          </Link>
        )}
      </div>

      {isToday && (
        <div className="mb-6 rounded border bg-white p-4">
          <h2 className="mb-2 text-lg font-semibold text-gray-800">{t('dailyReport.todayTicketsTitle')}</h2>
          {ticketsLoading ? (
            <p className="text-sm text-gray-500">{t('common.loading')}</p>
          ) : (
            <ul className="space-y-1 text-sm">
              {tickets?.map((ticket) => (
                <li key={ticket.id} className="text-gray-700">
                  #{ticket.id} {ticket.title}
                </li>
              ))}
              {tickets?.length === 0 && <li className="text-gray-400">{t('dailyReport.noTicketsToday')}</li>}
            </ul>
          )}
        </div>
      )}

      <div className="rounded border bg-white p-4">
        <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('dailyReport.alkptTitle')}</h2>

        {error && <div className="mb-3 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

        {existingReport?.locked ? (
          <div className="rounded bg-gray-50 px-3 py-3 text-sm text-gray-600">
            <p className="mb-2">
              {existingReport.lock_reason === 'approved'
                ? t('dailyReport.lockedApproved')
                : t('dailyReport.lockedMonthClosed')}
            </p>
            <Link to={`/daily-report/${date}`} className="text-blue-600 hover:underline">
              {t('dailyReport.backToView')}
            </Link>
          </div>
        ) : (
        <>
        <div className="space-y-3 text-sm" key={date}>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('dailyReport.achievement')}</span>
            <MarkdownField
              value={achievement}
              onChange={setAchievement}
              rows={6}
              startInPreview={!!existingReport?.achievement}
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('dailyReport.leftover')}</span>
            <MarkdownField
              value={leftover}
              onChange={setLeftover}
              rows={6}
              startInPreview={!!existingReport?.leftover}
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('dailyReport.keep')}</span>
            <MarkdownField value={keep} onChange={setKeep} rows={6} startInPreview={!!existingReport?.keep} />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('dailyReport.problem')}</span>
            <MarkdownField
              value={problem}
              onChange={setProblem}
              rows={6}
              startInPreview={!!existingReport?.problem}
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('dailyReport.try')}</span>
            <MarkdownField
              value={tryField}
              onChange={setTryField}
              rows={6}
              startInPreview={!!existingReport?.try}
            />
          </label>
        </div>

        <button
          onClick={handleSubmit}
          disabled={submitMutation.isPending}
          className="mt-4 rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('dailyReport.submit')}
        </button>
        </>
        )}
      </div>
    </div>
  )
}
