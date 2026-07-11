import { useEffect, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { endSession, getCurrent, startSession } from '../api/workSessions'
import { getErrorMessage } from '../i18n/apiErrors'

export function WorkSessionWidget() {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const queryClient = useQueryClient()

  const [now, setNow] = useState(() => new Date())
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 1000)
    return () => clearInterval(id)
  }, [])

  const { data: current, isLoading: currentLoading } = useQuery({
    queryKey: ['work-sessions', 'current'],
    queryFn: getCurrent,
    retry: false,
  })

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['work-sessions', 'current'] })
    queryClient.invalidateQueries({ queryKey: ['work-sessions', 'today'] })
  }

  const startMutation = useMutation({ mutationFn: startSession, onSuccess: invalidate })
  const endMutation = useMutation({ mutationFn: endSession, onSuccess: invalidate })

  const error = startMutation.error || endMutation.error
  const errorMessage = error ? getErrorMessage(error, t, 'workSession.actionFailed') : null

  const isWorking = !!current

  const buttonClass =
    'w-full rounded-lg py-6 text-xl font-semibold text-white shadow disabled:opacity-40 disabled:shadow-none'

  return (
    <div>
      <div className="mb-4 rounded border bg-white p-4 text-center">
        <p className="text-sm font-medium text-gray-500">
          {now.toLocaleDateString(locale, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'short' })}
        </p>
        <p className="font-mono text-6xl font-semibold tabular-nums text-gray-800">
          {now.toLocaleTimeString(locale, { hour12: false })}
        </p>
        {currentLoading ? (
          <p className="mt-1 text-sm text-gray-400">{t('common.loading')}</p>
        ) : (
          <p className={`mt-1 text-base font-medium ${isWorking ? 'text-blue-600' : 'text-gray-500'}`}>
            {isWorking ? t('workSession.currentlyWorking') : t('workSession.notWorking')}
          </p>
        )}
      </div>

      {errorMessage && (
        <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{errorMessage}</div>
      )}

      <div className="space-y-3">
        <button
          onClick={() => startMutation.mutate()}
          disabled={isWorking || startMutation.isPending}
          className={`${buttonClass} bg-blue-600 hover:bg-blue-700`}
        >
          {t('workSession.start')}
        </button>

        <button
          onClick={() => endMutation.mutate()}
          disabled={!isWorking || endMutation.isPending}
          className={`${buttonClass} bg-gray-700 hover:bg-gray-800`}
        >
          {t('workSession.end')}
        </button>
      </div>

      <p className="mt-4 text-center text-xs text-gray-400">
        {t('workSession.footerNoteBefore')}
        <Link to="/daily-report" className="underline">{t('workSession.footerLinkText')}</Link>
        {t('workSession.footerNoteAfter')}
      </p>
    </div>
  )
}
