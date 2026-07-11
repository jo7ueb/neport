import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { listTicketChildren } from '../api/tickets'
import { statusLabel } from '../i18n/statusLabel'
import type { Ticket, WorkflowStatus } from '../types/models'

function isOverdue(dueDate: string | null, status: WorkflowStatus): boolean {
  if (!dueDate) return false
  if (status.category === 'done') return false
  return dueDate < new Date().toISOString().slice(0, 10)
}

export function TicketRow({ ticket, depth = 0 }: { ticket: Ticket; depth?: number }) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false)

  const { data: children } = useQuery({
    queryKey: ['ticket', ticket.id, 'children'],
    queryFn: () => listTicketChildren(ticket.id),
    enabled: expanded,
  })

  return (
    <div>
      <div
        className={`flex items-center gap-2 rounded border px-2 py-1 text-sm ${
          isOverdue(ticket.due_date, ticket.status) ? 'bg-red-50' : 'bg-white'
        }`}
        style={{ marginLeft: depth * 16 }}
      >
        <button
          onClick={() => setExpanded((v) => !v)}
          className="w-5 text-gray-400 hover:text-gray-700"
          aria-label={t('ticket.row.expandAria')}
        >
          {expanded ? '▾' : '▸'}
        </button>
        <Link to={`/tickets/${ticket.id}`} className="flex-1 text-blue-600 hover:underline">
          #{ticket.id} {ticket.title}
        </Link>
        <span className="text-gray-500">{statusLabel(t, ticket.status)}</span>
        {ticket.due_date && (
          <span className={isOverdue(ticket.due_date, ticket.status) ? 'font-semibold text-red-700' : 'text-gray-400'}>
            {ticket.due_date}
          </span>
        )}
      </div>

      {expanded && (
        <div className="mt-1 space-y-1">
          {children?.map((child) => (
            <TicketRow key={child.id} ticket={child} depth={depth + 1} />
          ))}
          {children?.length === 0 && (
            <div className="text-xs text-gray-400" style={{ marginLeft: (depth + 1) * 16 }}>
              {t('ticket.row.noChildren')}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
