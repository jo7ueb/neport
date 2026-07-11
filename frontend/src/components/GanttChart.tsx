import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import Gantt, { type GanttTask } from 'frappe-gantt'
import { updateTicket } from '../api/tickets'
import { getErrorMessage } from '../i18n/apiErrors'
import type { Ticket, TicketDependency } from '../types/models'

// frappe-gantt's row layout, from its own default options (header_height /
// bar_height / padding - see setup_options in node_modules/frappe-gantt).
// We don't override any of those, so the ticket-name column built alongside
// the chart has to reproduce the same numbers to line its rows up with the
// bars. If the view options below ever change these, update this too.
const GANTT_HEADER_HEIGHT = 50
const GANTT_ROW_HEIGHT = 38 // padding (18) + bar_height (20)

interface HierarchyRow {
  ticket: Ticket
  depth: number
}

// Parent tickets are listed before their children, and a ticket's children
// are listed immediately after it (each ticket's own subtree kept together,
// depth-first) - rather than a flat sort that could scatter a parent from
// its children across the whole chart. Within a sibling group, oldest first.
function sortTicketsByHierarchy(tickets: Ticket[]): HierarchyRow[] {
  const byParent = new Map<number | null, Ticket[]>()
  for (const ticket of tickets) {
    const list = byParent.get(ticket.parent_ticket_id) ?? []
    list.push(ticket)
    byParent.set(ticket.parent_ticket_id, list)
  }
  for (const list of byParent.values()) {
    list.sort((a, b) => a.created_at.localeCompare(b.created_at))
  }

  const rows: HierarchyRow[] = []
  const visited = new Set<number>()
  function visit(ticket: Ticket, depth: number) {
    if (visited.has(ticket.id)) return
    visited.add(ticket.id)
    rows.push({ ticket, depth })
    for (const child of byParent.get(ticket.id) ?? []) {
      visit(child, depth + 1)
    }
  }
  for (const root of byParent.get(null) ?? []) {
    visit(root, 0)
  }
  // A ticket whose parent isn't in this list (shouldn't normally happen -
  // tickets and their parents are always in the same project) would
  // otherwise never be visited from the root loop above.
  for (const ticket of tickets) {
    visit(ticket, 0)
  }
  return rows
}

function addDays(dateStr: string, days: number): string {
  const d = new Date(dateStr)
  d.setDate(d.getDate() + days)
  return d.toISOString().slice(0, 10)
}

// frappe-gantt builds its bars from Date objects constructed with local
// year/month/day components (not UTC) - Date#toISOString() would shift
// across a timezone boundary, so dates read back off a dragged bar must be
// formatted from local components too.
function formatLocalDate(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

function formatShortDate(date: Date): string {
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${m}/${d}`
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function toTasks(rows: HierarchyRow[], dependencies: TicketDependency[]): GanttTask[] {
  const predecessorsBySuccessor = new Map<number, number[]>()
  for (const dep of dependencies) {
    const list = predecessorsBySuccessor.get(dep.successor_ticket_id) ?? []
    list.push(dep.predecessor_ticket_id)
    predecessorsBySuccessor.set(dep.successor_ticket_id, list)
  }

  return rows.map(({ ticket }) => {
    const start = ticket.start_date ?? ticket.created_at.slice(0, 10)
    let end = ticket.end_date ?? ticket.due_date ?? start
    if (end <= start) {
      end = addDays(start, 1)
    }

    const predecessors = predecessorsBySuccessor.get(ticket.id) ?? []

    return {
      id: String(ticket.id),
      name: `#${ticket.id} ${ticket.title}`,
      start,
      end,
      progress: ticket.status.category === 'done' ? 100 : ticket.status.category === 'in_progress' ? 50 : 0,
      dependencies: predecessors.map(String).join(','),
    }
  })
}

export function GanttChart({
  projectId,
  tickets,
  dependencies,
}: {
  projectId: number
  tickets: Ticket[]
  dependencies: TicketDependency[]
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const containerRef = useRef<HTMLDivElement>(null)
  const ganttRef = useRef<Gantt | null>(null)
  const [error, setError] = useState<string | null>(null)

  const updateMutation = useMutation({
    mutationFn: ({ ticketId, start, end }: { ticketId: number; start: string; end: string }) =>
      updateTicket(ticketId, { start_date: start, end_date: end }),
    onSuccess: () => {
      setError(null)
      queryClient.invalidateQueries({ queryKey: ['gantt', projectId] })
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'gantt.updateFailed'))
      // The bar was already dragged visually - force a refresh from the
      // server state so it snaps back after a rejected change.
      queryClient.invalidateQueries({ queryKey: ['gantt', projectId] })
    },
  })
  // The Gantt instance (and the callbacks it was constructed with) is only
  // created once - route date-change events through a ref so they always
  // reach the latest mutation rather than the one from the first render.
  const mutateRef = useRef(updateMutation.mutate)
  mutateRef.current = updateMutation.mutate

  // Shared between the SVG bars and the ticket-name column rendered
  // alongside them, so both agree on row order/count.
  const hierarchyRows = sortTicketsByHierarchy(tickets)

  useEffect(() => {
    if (!containerRef.current || tickets.length === 0) return

    const tasks = toTasks(hierarchyRows, dependencies)

    if (ganttRef.current) {
      ganttRef.current.refresh(tasks)
    } else {
      ganttRef.current = new Gantt(containerRef.current, tasks, {
        view_mode: 'Day',
        on_click: (task) => navigate(`/tickets/${task.id}`),
        on_date_change: (task, start, end) => {
          mutateRef.current({
            ticketId: Number(task.id),
            start: formatLocalDate(start),
            end: formatLocalDate(end),
          })
        },
        custom_popup_html: (task) => {
          const start = formatShortDate(task._start!)
          // task._end is exclusive (bumped a day past the last visible day
          // when no time-of-day was set) - pull back a second the same way
          // frappe-gantt's own default popup does before formatting.
          const end = formatShortDate(new Date(task._end!.getTime() - 1000))
          return `
            <div class="title">${escapeHtml(task.name)}</div>
            <div class="subtitle">${start} - ${end}</div>
            <a href="/tickets/${task.id}" style="display:block;margin-top:4px;color:#2563eb;font-size:12px;text-decoration:underline;">
              ${escapeHtml(t('gantt.openTicket'))}
            </a>
          `
        },
      })
    }
  }, [tickets, dependencies, navigate, t])

  if (tickets.length === 0) {
    return <p className="text-sm text-gray-400">{t('gantt.empty')}</p>
  }

  return (
    <div>
      {error && <div className="mb-2 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
      <div className="flex items-start">
        <div className="w-56 flex-shrink-0 overflow-hidden border-r border-gray-200">
          {/* Spacer to line the first name row up with the chart's date header. */}
          <div style={{ height: GANTT_HEADER_HEIGHT }} />
          {hierarchyRows.map(({ ticket, depth }) => (
            <div
              key={ticket.id}
              style={{ height: GANTT_ROW_HEIGHT, paddingLeft: 8 + depth * 14 }}
              className="flex items-center overflow-hidden text-ellipsis whitespace-nowrap pr-2 text-xs"
              title={`#${ticket.id} ${ticket.title}`}
            >
              <Link to={`/tickets/${ticket.id}`} className="truncate text-blue-600 hover:underline">
                #{ticket.id} {ticket.title}
              </Link>
            </div>
          ))}
        </div>
        <div ref={containerRef} className="gantt-container flex-1 overflow-x-auto" />
      </div>
    </div>
  )
}
