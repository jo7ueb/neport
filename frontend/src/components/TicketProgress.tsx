import { useTranslation } from 'react-i18next'

// Shows a parent ticket's progress as the share of its whole subtree
// (children, grandchildren, ...) that are done - renders nothing for a
// childless ticket (there's no ratio to show).
export function TicketProgress({
  done,
  total,
  className = '',
}: {
  done: number
  total: number
  className?: string
}) {
  const { t } = useTranslation()
  if (total === 0) return null

  const percent = Math.round((done / total) * 100)

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <div className="h-1.5 w-16 overflow-hidden rounded-full bg-gray-200">
        <div className="h-full rounded-full bg-green-500" style={{ width: `${percent}%` }} />
      </div>
      <span className="whitespace-nowrap text-xs text-gray-500">
        {t('ticket.progress.fraction', { done, total, percent })}
      </span>
    </div>
  )
}
