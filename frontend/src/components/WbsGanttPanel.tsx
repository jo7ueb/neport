import { useEffect, useMemo, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import Gantt, { type GanttTask } from 'frappe-gantt'
import { updateTicket, type UpdateTicketInput } from '../api/tickets'
import { getErrorMessage } from '../i18n/apiErrors'
import { statusLabel } from '../i18n/statusLabel'
import { TicketCreateModal } from './TicketCreateModal'
import { STORY_POINT_OPTIONS } from '../lib/storyPoints'
import type { Ticket, TicketDependency, TicketNode, User, WorkflowDetail, WorkflowStatus } from '../types/models'

// Explicit, rather than relying on frappe-gantt's un-overridden Day-view
// defaults - passed into `new Gantt(...)` below so the WBS panel's row/header
// heights can be pinned to these exact same numbers and stay lined up with
// the gantt bars.
const BAR_HEIGHT = 20
const ROW_PADDING = 18
const ROW_HEIGHT = BAR_HEIGHT + ROW_PADDING
const HEADER_HEIGHT = 50

interface FlatRow {
  ticket: Ticket
  depth: number
  prevSiblingId: number | null
  wbsNumber: string
}

function flatten(nodes: TicketNode[], depth = 0, parentNumber = ''): FlatRow[] {
  const rows: FlatRow[] = []
  let prevSiblingId: number | null = null
  for (const [index, node] of nodes.entries()) {
    const { children, ...ticket } = node
    const wbsNumber = parentNumber ? `${parentNumber}.${index + 1}` : String(index + 1)
    rows.push({ ticket, depth, prevSiblingId, wbsNumber })
    rows.push(...flatten(children, depth + 1, wbsNumber))
    prevSiblingId = node.id
  }
  return rows
}

type WbsSort = 'manual' | 'title_asc' | 'due_date_asc' | 'created_at_asc' | 'updated_at_desc' | 'status_asc'

function sortNodes(nodes: TicketNode[], sort: WbsSort): TicketNode[] {
  if (sort === 'manual') return nodes

  const sorted = [...nodes].sort((a, b) => {
    switch (sort) {
      case 'title_asc':
        return a.title.localeCompare(b.title, undefined, { sensitivity: 'base' })
      case 'due_date_asc':
        return (a.due_date ?? '9999-12-31').localeCompare(b.due_date ?? '9999-12-31')
      case 'created_at_asc':
        return a.created_at.localeCompare(b.created_at)
      case 'updated_at_desc':
        return b.updated_at.localeCompare(a.updated_at)
      case 'status_asc':
        return a.status.position - b.status.position || a.title.localeCompare(b.title, undefined, { sensitivity: 'base' })
    }
  })

  return sorted.map((node) => ({ ...node, children: sortNodes(node.children, sort) }))
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

function toTasks(rows: FlatRow[], dependencies: TicketDependency[]): GanttTask[] {
  const predecessorsBySuccessor = new Map<number, number[]>()
  for (const dep of dependencies) {
    const list = predecessorsBySuccessor.get(dep.successor_ticket_id) ?? []
    list.push(dep.predecessor_ticket_id)
    predecessorsBySuccessor.set(dep.successor_ticket_id, list)
  }

  return rows.map(({ ticket }) => {
    const start = ticket.start_date ?? ticket.created_at.slice(0, 10)
    // The bar's end represents the due date, not end_date (which nothing
    // else in the UI ever sets - see the on_date_change handler below,
    // which saves a drag as due_date). end_date is only consulted as a
    // fallback for older data saved before that was true.
    let end = ticket.due_date ?? ticket.end_date ?? start
    if (end <= start) {
      end = addDays(start, 1)
    }

    const predecessors = predecessorsBySuccessor.get(ticket.id) ?? []

    return {
      id: String(ticket.id),
      name: `#${ticket.id} ${ticket.title}`,
      start,
      end,
      // "done" only means the assignee claims it's finished - a project
      // manager hasn't reviewed it yet, so it shows as nearly-but-not-quite
      // complete. Only "confirmed" (closed after review) reaches 100%.
      progress:
        ticket.status.category === 'confirmed'
          ? 100
          : ticket.status.category === 'done'
            ? 90
            : ticket.status.category === 'in_progress'
              ? 50
              : 0,
      dependencies: predecessors.map(String).join(','),
    }
  })
}

// frappe-gantt has no built-in weekend/holiday highlighting option (only a
// "today" highlight). `dates`/`options`/`tasks`/`layers` are plain (non-
// private) instance properties, so we inject our own SVG rects into the grid
// layer using the same geometry frappe-gantt itself uses for the today
// highlight. `render()` (called from both the constructor and `refresh()`)
// clears and rebuilds every layer each time, so this must be re-applied after
// every call, not just once.
function applyWeekendHighlights(gantt: Gantt) {
  const gridHeight =
    gantt.options.header_height +
    gantt.options.padding +
    (gantt.options.bar_height + gantt.options.padding) * gantt.tasks.length
  const todayHighlight = gantt.layers.grid.querySelector('.today-highlight')

  gantt.dates.forEach((date, i) => {
    const day = date.getDay()
    if (day !== 0 && day !== 6) return
    const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect')
    rect.setAttribute('x', String(i * gantt.options.column_width))
    rect.setAttribute('y', '0')
    rect.setAttribute('width', String(gantt.options.column_width))
    rect.setAttribute('height', String(gridHeight))
    rect.setAttribute('class', 'weekend-highlight')
    // Inserted before the today-highlight (if any) so today's own highlight
    // stays visible on top when today happens to fall on a weekend.
    if (todayHighlight) {
      gantt.layers.grid.insertBefore(rect, todayHighlight)
    } else {
      gantt.layers.grid.appendChild(rect)
    }
  })
}

// frappe-gantt's month header (Day view's "upper text") only ships English
// (plus a handful of other languages, none of them Japanese - see
// month_names in node_modules/frappe-gantt/src/date_utils.js) and setting
// `options.language` to an unlisted code throws (it indexes straight into
// that table). So instead of asking frappe-gantt to localize, we let it
// render in English and then relabel the month headers ourselves from the
// app's own i18n locale. `x` on each `.upper-text` node is
// `index * column_width + column_width * 15` (Day_upper's x_pos, see
// get_date_info() in frappe-gantt's index.js) - invert that to recover which
// `gantt.dates[index]` produced it, then reformat that date with Intl.
function localizeMonthHeaders(gantt: Gantt, locale: string) {
  const formatter = new Intl.DateTimeFormat(locale, { month: 'long' })
  const xOffset = gantt.options.column_width * 15

  gantt.layers.date.querySelectorAll<SVGTextElement>('.upper-text').forEach((el) => {
    const x = Number(el.getAttribute('x'))
    const index = Math.round((x - xOffset) / gantt.options.column_width)
    const date = gantt.dates[index]
    if (date) {
      el.textContent = formatter.format(date)
    }
  })
}

type EditableField = 'title' | 'status' | 'assignee_id' | 'story_points'

export function WbsGanttPanel({
  projectId,
  nodes,
  dependencies,
  users,
  workflow,
}: {
  projectId: number
  nodes: TicketNode[]
  dependencies: TicketDependency[]
  users: User[]
  workflow?: WorkflowDetail
}) {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const containerRef = useRef<HTMLDivElement>(null)
  const ganttRef = useRef<Gantt | null>(null)
  const [chartError, setChartError] = useState<string | null>(null)
  const [editingCell, setEditingCell] = useState<{ ticketId: number; field: EditableField } | null>(null)
  const [draftValue, setDraftValue] = useState('')
  const [sort, setSort] = useState<WbsSort>('manual')
  // undefined = modal closed, null = creating a root task, number = creating a child of that ticket
  const [createModalParent, setCreateModalParent] = useState<number | null | undefined>(undefined)

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['project', projectId, 'tickets', 'tree'] })

  const sortedNodes = useMemo(() => sortNodes(nodes, sort), [nodes, sort])
  const rows = flatten(sortedNodes)
  const rowsById = new Map(rows.map((r) => [r.ticket.id, r]))

  const updateMutation = useMutation({
    mutationFn: ({ id, input }: { id: number; input: UpdateTicketInput }) => updateTicket(id, input),
    onSuccess: invalidate,
  })

  const dateMutation = useMutation({
    mutationFn: ({ ticketId, start, end }: { ticketId: number; start: string; end: string }) =>
      updateTicket(ticketId, { start_date: start, due_date: end }),
    onSuccess: () => {
      setChartError(null)
      invalidate()
    },
    onError: (err) => {
      setChartError(getErrorMessage(err, t, 'gantt.updateFailed'))
      // The bar was already dragged visually - force a refresh from the
      // server state so it snaps back after a rejected change.
      invalidate()
    },
  })
  // The Gantt instance (and the callbacks it was constructed with) is only
  // created once - route date-change events through a ref so they always
  // reach the latest mutation rather than the one from the first render.
  const dateMutateRef = useRef(dateMutation.mutate)
  dateMutateRef.current = dateMutation.mutate

  function startEdit(ticket: Ticket, field: EditableField) {
    if (field === 'story_points' && ticket.child_ticket_count > 0) return
    if (field === 'status') {
      setDraftValue(String(ticket.status.id))
    } else if (field === 'assignee_id') {
      setDraftValue(ticket.assignee_id === null ? '' : String(ticket.assignee_id))
    } else if (field === 'story_points') {
      setDraftValue(ticket.story_points === null ? '' : String(ticket.story_points))
    } else {
      setDraftValue(ticket.title)
    }
    setEditingCell({ ticketId: ticket.id, field })
  }

  function commitEdit() {
    if (!editingCell) return
    const { ticketId, field } = editingCell
    const input: UpdateTicketInput = {}
    if (field === 'assignee_id') {
      input.assignee_id = draftValue === '' ? null : Number(draftValue)
    } else if (field === 'story_points') {
      input.story_points = draftValue === '' ? null : Number(draftValue)
    } else if (field === 'status') {
      input.status_id = Number(draftValue)
    } else {
      input.title = draftValue
    }
    updateMutation.mutate({ id: ticketId, input })
    setEditingCell(null)
  }

  function cancelEdit() {
    setEditingCell(null)
  }

  // Same "current status + whatever the workflow allows moving to" rule as
  // the ticket detail page's status dropdown.
  function statusOptionsFor(currentStatus: WorkflowStatus): WorkflowStatus[] {
    if (!workflow) return [currentStatus]
    const reachable = new Set(
      workflow.transitions.filter((tr) => tr.from_status_id === currentStatus.id).map((tr) => tr.to_status_id)
    )
    return workflow.statuses.filter((s) => s.id === currentStatus.id || reachable.has(s.id))
  }

  function indent(row: FlatRow) {
    if (row.prevSiblingId === null) return
    updateMutation.mutate({ id: row.ticket.id, input: { parent_ticket_id: row.prevSiblingId } })
  }

  function outdent(row: FlatRow) {
    if (row.ticket.parent_ticket_id === null) return
    const parentRow = rowsById.get(row.ticket.parent_ticket_id)
    const newParentId = parentRow ? parentRow.ticket.parent_ticket_id : null
    updateMutation.mutate({ id: row.ticket.id, input: { parent_ticket_id: newParentId } })
  }

  function progressLabel(ticket: Ticket): string {
    if (ticket.child_ticket_count === 0) return '-'
    return `${Math.round((ticket.child_done_ticket_count / ticket.child_ticket_count) * 100)}%`
  }

  function userLabel(id: number | null): string {
    if (id === null) return t('wbs.unassigned')
    return users.find((u) => u.id === id)?.display_name ?? `#${id}`
  }

  useEffect(() => {
    if (!containerRef.current || rows.length === 0) return

    const tasks = toTasks(rows, dependencies)

    if (ganttRef.current) {
      ganttRef.current.refresh(tasks)
    } else {
      ganttRef.current = new Gantt(containerRef.current, tasks, {
        view_mode: 'Day',
        bar_height: BAR_HEIGHT,
        padding: ROW_PADDING,
        header_height: HEADER_HEIGHT,
        on_click: (task) => navigate(`/tickets/${task.id}`),
        on_date_change: (task, start, end) => {
          dateMutateRef.current({
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
    applyWeekendHighlights(ganttRef.current)
    localizeMonthHeaders(ganttRef.current, i18n.language)
    // rows/dependencies carry all the task data the effect depends on; t is
    // only read inside callbacks invoked later. Re-running per render would
    // rebuild the whole chart for no reason.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, dependencies, navigate, i18n.language])

  return (
    <div>
      {chartError && <div className="mb-2 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{chartError}</div>}

      <div className="mb-2 flex flex-wrap items-center gap-2">
        <button
          onClick={() => setCreateModalParent(null)}
          className="rounded border border-gray-300 px-3 py-1 text-sm text-gray-700 hover:bg-gray-100"
        >
          {t('wbs.addRootTask')}
        </button>
        <label className="ml-auto flex items-center gap-2 text-sm text-gray-600">
          <span>{t('wbs.sortLabel')}</span>
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value as WbsSort)}
            className="rounded border border-gray-300 bg-white px-2 py-1 text-sm text-gray-700"
          >
            <option value="manual">{t('wbs.sort.manual')}</option>
            <option value="title_asc">{t('wbs.sort.titleAsc')}</option>
            <option value="due_date_asc">{t('wbs.sort.dueDateAsc')}</option>
            <option value="created_at_asc">{t('wbs.sort.createdAtAsc')}</option>
            <option value="updated_at_desc">{t('wbs.sort.updatedAtDesc')}</option>
            <option value="status_asc">{t('wbs.sort.statusAsc')}</option>
          </select>
        </label>
      </div>

      {rows.length === 0 ? (
        <p className="text-sm text-gray-400">{t('wbs.empty')}</p>
      ) : (
        <div className="flex items-start overflow-hidden rounded border bg-white">
          <div className="w-[52%] min-w-[900px] max-w-[1200px] flex-shrink-0 overflow-hidden border-r border-gray-200 text-sm">
            <div
              style={{ height: HEADER_HEIGHT }}
              className="flex items-center border-b bg-gray-50 px-2 text-xs font-medium text-gray-500"
            >
              <span className="w-12 flex-shrink-0">{t('wbs.columns.hierarchy')}</span>
              <span className="w-16 flex-shrink-0">{t('wbs.columns.wbsNumber')}</span>
              <span className="min-w-0 flex-1">{t('wbs.columns.title')}</span>
              <span className="w-20 flex-shrink-0">{t('wbs.columns.startDate')}</span>
              <span className="w-20 flex-shrink-0">{t('wbs.columns.dueDate')}</span>
              <span className="w-14 flex-shrink-0">{t('wbs.columns.storyPoints')}</span>
              <span className="w-24 flex-shrink-0">{t('wbs.columns.status')}</span>
              <span className="w-12 flex-shrink-0">{t('wbs.columns.progress')}</span>
              <span className="w-32 flex-shrink-0">{t('wbs.columns.assignee')}</span>
              <span className="w-6 flex-shrink-0" />
            </div>
            {rows.map((row) => {
              const { ticket, depth } = row
              const isEditing = (field: EditableField) =>
                editingCell?.ticketId === ticket.id && editingCell.field === field

              return (
                <div
                  key={ticket.id}
                  style={{ height: ROW_HEIGHT }}
                  className={`flex items-center border-b border-gray-100 px-2 hover:bg-gray-50 ${
                    ticket.child_ticket_count > 0 ? 'bg-slate-50/60' : ''
                  }`}
                >
                  <span className="flex w-12 flex-shrink-0 items-center">
                    <button
                      onClick={() => outdent(row)}
                      disabled={depth === 0}
                      className="text-gray-400 hover:text-gray-700 disabled:opacity-30"
                      aria-label={t('wbs.outdentAria')}
                    >
                      ←
                    </button>
                    <button
                      onClick={() => indent(row)}
                      disabled={row.prevSiblingId === null}
                      className="text-gray-400 hover:text-gray-700 disabled:opacity-30"
                      aria-label={t('wbs.indentAria')}
                    >
                      →
                    </button>
                  </span>
                  <span className="w-16 flex-shrink-0 font-mono text-xs text-gray-500">{row.wbsNumber}</span>
                  <span
                    className={`relative min-w-0 flex-1 ${ticket.child_ticket_count > 0 ? 'font-medium' : ''}`}
                    style={{ paddingLeft: depth * 18 + 4 }}
                  >
                    {Array.from({ length: depth }, (_, guideDepth) => (
                      <span
                        key={guideDepth}
                        aria-hidden="true"
                        className="absolute bottom-0 top-0 border-l border-slate-200"
                        style={{ left: guideDepth * 18 + 8 }}
                      />
                    ))}
                    {depth > 0 && (
                      <span
                        aria-hidden="true"
                        className="absolute top-1/2 w-3 border-t border-slate-300"
                        style={{ left: (depth - 1) * 18 + 8 }}
                      />
                    )}
                    {isEditing('title') ? (
                      <input
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') commitEdit()
                          if (e.key === 'Escape') cancelEdit()
                        }}
                        className="w-full rounded border border-gray-300 px-1 py-0.5 text-sm"
                      />
                    ) : (
                      <span
                        onClick={() => startEdit(ticket, 'title')}
                        className="block cursor-text truncate hover:bg-gray-50"
                        title={`#${ticket.id} ${ticket.title}`}
                      >
                        <Link
                          to={`/tickets/${ticket.id}`}
                          onClick={(e) => e.stopPropagation()}
                          className="text-blue-600 hover:underline"
                        >
                          #{ticket.id}
                        </Link>{' '}
                        {ticket.title}
                      </span>
                    )}
                  </span>
                  <span className="w-20 flex-shrink-0 truncate text-xs text-gray-600">{ticket.start_date ?? '-'}</span>
                  <span className="w-20 flex-shrink-0 truncate text-xs text-gray-600">{ticket.due_date ?? '-'}</span>
                  <span className="w-14 flex-shrink-0 text-center text-xs text-gray-600">
                    {isEditing('story_points') ? (
                      <select
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        className="w-full rounded border border-gray-300 px-1 py-0.5 text-xs"
                      >
                        <option value="">-</option>
                        {STORY_POINT_OPTIONS.map((point) => (
                          <option key={point} value={point}>
                            {point}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span
                        onClick={() => startEdit(ticket, 'story_points')}
                        className={ticket.child_ticket_count > 0 ? '' : 'cursor-pointer hover:bg-gray-50'}
                      >
                        {ticket.child_ticket_count > 0 ? t('ticket.detail.storyPointsNotApplicable') : ticket.story_points ?? '-'}
                      </span>
                    )}
                  </span>
                  <span className="w-24 flex-shrink-0 truncate">
                    {isEditing('status') ? (
                      <select
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        className="w-full rounded border border-gray-300 px-1 py-0.5 text-xs"
                      >
                        {statusOptionsFor(ticket.status).map((s) => (
                          <option key={s.id} value={s.id}>
                            {statusLabel(t, s)}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span onClick={() => startEdit(ticket, 'status')} className="cursor-pointer hover:bg-gray-50">
                        {statusLabel(t, ticket.status)}
                      </span>
                    )}
                  </span>
                  <span className="w-12 flex-shrink-0 text-xs text-gray-500">{progressLabel(ticket)}</span>
                  <span className="w-32 flex-shrink-0 truncate">
                    {isEditing('assignee_id') ? (
                      <select
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        className="w-full rounded border border-gray-300 px-1 py-0.5 text-xs"
                      >
                        <option value="">{t('wbs.unassigned')}</option>
                        {users.map((u) => (
                          <option key={u.id} value={u.id}>
                            {u.display_name}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span
                        onClick={() => startEdit(ticket, 'assignee_id')}
                        className="cursor-pointer hover:bg-gray-50"
                      >
                        {userLabel(ticket.assignee_id)}
                      </span>
                    )}
                  </span>
                  <span className="w-6 flex-shrink-0 text-right">
                    <button
                      onClick={() => setCreateModalParent(ticket.id)}
                      className="text-blue-600 hover:underline"
                      aria-label={t('wbs.addChildTask')}
                    >
                      +
                    </button>
                  </span>
                </div>
              )
            })}
          </div>
          <div ref={containerRef} className="gantt-container flex-1 overflow-x-auto" />
        </div>
      )}

      <TicketCreateModal
        open={createModalParent !== undefined}
        onClose={() => setCreateModalParent(undefined)}
        projectId={projectId}
        parentTicketId={createModalParent ?? undefined}
        onCreated={invalidate}
      />
    </div>
  )
}
