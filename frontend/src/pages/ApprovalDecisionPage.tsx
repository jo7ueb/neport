import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { decide, getSubordinateDay, listSubordinates } from '../api/approvals'
import { MarkdownContent } from '../components/MarkdownContent'
import { getErrorMessage } from '../i18n/apiErrors'

export function ApprovalDecisionPage() {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const { userId: userIdParam, date } = useParams()
  const userId = Number(userIdParam)
  const queryClient = useQueryClient()

  const [comment, setComment] = useState('')
  const [error, setError] = useState<string | null>(null)

  const { data: subordinates } = useQuery({ queryKey: ['approvals', 'subordinates'], queryFn: listSubordinates })
  const subject = subordinates?.find((u) => u.id === userId)

  const { data, isLoading } = useQuery({
    queryKey: ['approvals', 'subordinates', userId, 'day', date],
    queryFn: () => getSubordinateDay(userId, date!),
    enabled: !!date,
  })

  const decideMutation = useMutation({
    mutationFn: (decision: 'approved' | 'rejected') => decide(userId, date!, decision, comment),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['approvals', 'subordinates', userId, 'day', date] })
      queryClient.invalidateQueries({ queryKey: ['approvals', 'subordinates', userId, 'monthly'] })
      setError(null)
      setComment('')
    },
    onError: (err) => setError(getErrorMessage(err, t, 'approvals.decideFailed')),
  })

  const formattedDate = date
    ? new Date(`${date}T00:00:00`).toLocaleDateString(locale, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'short' })
    : ''

  const canDecide = data?.status === 'valid'
  const canRevoke = data?.status === 'approved'

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {subject ? t('approvals.decisionTitleFor', { name: subject.display_name, date: formattedDate }) : formattedDate}
        </h1>
        <Link to={`/approvals/${userId}/monthly`} className="text-sm text-blue-600 hover:underline">
          {t('approvals.backToMonthly')}
        </Link>
      </div>

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <>
          <div className="mb-4 rounded border bg-white p-4">
            <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('dailyReport.alkptTitle')}</h2>
            {data?.report ? (
              <div className="space-y-4 text-sm">
                <div>
                  <p className="mb-1 font-medium text-gray-600">{t('dailyReport.achievement')}</p>
                  <MarkdownContent text={data.report.achievement || t('markdown.emptyPreview')} />
                </div>
                <div>
                  <p className="mb-1 font-medium text-gray-600">{t('dailyReport.leftover')}</p>
                  <MarkdownContent text={data.report.leftover || t('markdown.emptyPreview')} />
                </div>
                <div>
                  <p className="mb-1 font-medium text-gray-600">{t('dailyReport.keep')}</p>
                  <MarkdownContent text={data.report.keep || t('markdown.emptyPreview')} />
                </div>
                <div>
                  <p className="mb-1 font-medium text-gray-600">{t('dailyReport.problem')}</p>
                  <MarkdownContent text={data.report.problem || t('markdown.emptyPreview')} />
                </div>
                <div>
                  <p className="mb-1 font-medium text-gray-600">{t('dailyReport.try')}</p>
                  <MarkdownContent text={data.report.try || t('markdown.emptyPreview')} />
                </div>
              </div>
            ) : (
              <p className="text-sm text-gray-400">{t('approvals.noReportYet')}</p>
            )}
          </div>

          <div className="mb-4 rounded border bg-white p-4">
            <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('approvals.decisionSectionTitle')}</h2>

            {error && <div className="mb-3 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

            {canDecide ? (
              <div className="space-y-3 text-sm">
                <label className="block">
                  <span className="mb-1 block text-gray-600">{t('approvals.commentLabel')}</span>
                  <textarea
                    value={comment}
                    onChange={(e) => setComment(e.target.value)}
                    rows={3}
                    className="w-full rounded border border-gray-300 px-3 py-2 text-sm"
                  />
                </label>
                <div className="flex gap-3">
                  <button
                    onClick={() => decideMutation.mutate('approved')}
                    disabled={decideMutation.isPending}
                    className="rounded bg-green-600 px-4 py-2 text-white hover:bg-green-700 disabled:opacity-50"
                  >
                    {t('approvals.approve')}
                  </button>
                  <button
                    onClick={() => decideMutation.mutate('rejected')}
                    disabled={decideMutation.isPending}
                    className="rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700 disabled:opacity-50"
                  >
                    {t('approvals.reject')}
                  </button>
                </div>
              </div>
            ) : canRevoke ? (
              <div className="space-y-3 text-sm">
                <p className="text-gray-500">{t('approvals.revokeNotice')}</p>
                <label className="block">
                  <span className="mb-1 block text-gray-600">{t('approvals.commentLabel')}</span>
                  <textarea
                    value={comment}
                    onChange={(e) => setComment(e.target.value)}
                    rows={3}
                    className="w-full rounded border border-gray-300 px-3 py-2 text-sm"
                  />
                </label>
                <button
                  onClick={() => decideMutation.mutate('rejected')}
                  disabled={decideMutation.isPending}
                  className="rounded bg-orange-600 px-4 py-2 text-white hover:bg-orange-700 disabled:opacity-50"
                >
                  {t('approvals.revoke')}
                </button>
              </div>
            ) : (
              <p className="text-sm text-gray-500">
                {data?.status ? t(`approvals.notDecidableStatus.${data.status}` as const) : t('approvals.noReportYet')}
              </p>
            )}
          </div>

          <div className="rounded border bg-white p-4">
            <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('approvals.historyTitle')}</h2>
            {data?.history.length ? (
              <ul className="space-y-2 text-sm">
                {data.history.map((h, i) => (
                  <li key={i} className="rounded border px-3 py-2">
                    <div className="flex items-center justify-between">
                      <span className={h.decision === 'approved' ? 'font-medium text-green-700' : 'font-medium text-red-700'}>
                        {t(`workSession.dayStatus.${h.decision}` as const)}
                      </span>
                      <span className="text-xs text-gray-500">
                        {h.approver_name} · {new Date(h.decided_at).toLocaleString(locale)}
                      </span>
                    </div>
                    {h.comment && <p className="mt-1 text-gray-700">{h.comment}</p>}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-sm text-gray-400">{t('approvals.noHistory')}</p>
            )}
          </div>
        </>
      )}
    </div>
  )
}
