import { useTranslation } from 'react-i18next'

const DEFAULT_PER_PAGE_OPTIONS = [10, 25, 50, 100]

export function Pagination({
  page,
  perPage,
  total,
  onPageChange,
  onPerPageChange,
  perPageOptions = DEFAULT_PER_PAGE_OPTIONS,
}: {
  page: number
  perPage: number
  total: number
  onPageChange: (page: number) => void
  onPerPageChange: (perPage: number) => void
  perPageOptions?: number[]
}) {
  const { t } = useTranslation()
  const totalPages = Math.max(1, Math.ceil(total / perPage))
  const rangeStart = total === 0 ? 0 : (page - 1) * perPage + 1
  const rangeEnd = Math.min(page * perPage, total)

  return (
    <div className="mt-3 flex flex-wrap items-center justify-between gap-2 text-sm text-gray-600">
      <div>
        {total === 0
          ? t('pagination.zeroItems')
          : t('pagination.rangeCount', { start: rangeStart, end: rangeEnd, total })}
      </div>
      <div className="flex items-center gap-2">
        <label className="flex items-center gap-1">
          {t('pagination.perPage')}
          <select
            value={perPage}
            onChange={(e) => onPerPageChange(Number(e.target.value))}
            className="rounded border border-gray-300 px-2 py-1"
          >
            {perPageOptions.map((n) => (
              <option key={n} value={n}>
                {t('pagination.perPageOption', { count: n })}
              </option>
            ))}
          </select>
        </label>
        <button
          onClick={() => onPageChange(page - 1)}
          disabled={page <= 1}
          className="rounded border border-gray-300 px-2 py-1 hover:bg-gray-100 disabled:opacity-30"
        >
          {t('pagination.prev')}
        </button>
        <span>
          {page} / {totalPages}
        </span>
        <button
          onClick={() => onPageChange(page + 1)}
          disabled={page >= totalPages}
          className="rounded border border-gray-300 px-2 py-1 hover:bg-gray-100 disabled:opacity-30"
        >
          {t('pagination.next')}
        </button>
      </div>
    </div>
  )
}
