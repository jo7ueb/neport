import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { DndContext, useDraggable, useDroppable, type DragEndEvent, type DragStartEvent } from '@dnd-kit/core'
import { getProject, getProjectWorkflow } from '../api/projects'
import { listMilestones } from '../api/milestones'
import { listAllProjectTickets, updateTicket } from '../api/tickets'
import { listAllUsers } from '../api/users'
import { ProjectNav } from '../components/ProjectNav'
import { getErrorMessage } from '../i18n/apiErrors'
import { useProjectRole } from '../lib/useProjectRole'
import { statusLabel } from '../i18n/statusLabel'
import type { Ticket, WorkflowDetail, WorkflowStatus } from '../types/models'

// Mirrors TicketListPage's own isOverdue - a ticket in a done/confirmed
// status is never "overdue" regardless of its due date.
function isOverdue(dueDate: string | null, status: WorkflowStatus): boolean {
  if (!dueDate) return false
  if (status.category === 'done' || status.category === 'confirmed') return false
  return dueDate < new Date().toISOString().slice(0, 10)
}

function TicketCard({ ticket }: { ticket: Ticket }) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: ticket.id,
  })

  const style = transform
    ? { transform: `translate(${transform.x}px, ${transform.y}px)`, zIndex: 10 }
    : undefined

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      className={`cursor-grab rounded border bg-white px-3 py-2 text-sm shadow-sm ${isDragging ? 'opacity-50' : ''}`}
    >
      <Link to={`/tickets/${ticket.id}`} onClick={(e) => e.stopPropagation()} className="text-blue-600 hover:underline">
        #{ticket.id} {ticket.title}
      </Link>
    </div>
  )
}

function KanbanColumnDropzone({
  status,
  tickets,
  invalidTarget,
}: {
  status: WorkflowStatus
  tickets: Ticket[]
  // true while a card is being dragged and this column isn't a status the
  // dragged ticket is allowed to move into.
  invalidTarget: boolean
}) {
  const { t } = useTranslation()
  const { setNodeRef, isOver } = useDroppable({ id: status.id })

  return (
    <div
      ref={setNodeRef}
      className={`min-h-[200px] w-64 flex-shrink-0 rounded border p-2 ${
        invalidTarget ? 'cursor-not-allowed bg-gray-100 opacity-50' : 'bg-gray-50'
      } ${isOver && !invalidTarget ? 'ring-2 ring-blue-400' : ''} ${isOver && invalidTarget ? 'ring-2 ring-red-300' : ''}`}
    >
      <h3 className="mb-2 text-sm font-semibold text-gray-700">{statusLabel(t, status)}</h3>
      <div className="space-y-2">
        {tickets.map((ticket) => (
          <TicketCard key={ticket.id} ticket={ticket} />
        ))}
      </div>
    </div>
  )
}

// A status is a valid drop target for a ticket currently on `fromStatusId`
// if it's the same column (no-op) or the workflow declares a transition
// between them. Dropping onto a "confirmed"-category column (closing a done
// ticket after review) additionally requires project-manager access - the
// backend enforces the same rule, this just avoids a drag that's guaranteed
// to bounce back as a 403.
function isReachable(
  workflow: WorkflowDetail | undefined,
  fromStatusId: number,
  toStatus: WorkflowStatus,
  canManage: boolean
): boolean {
  if (fromStatusId === toStatus.id) return true
  if (toStatus.category === 'confirmed' && !canManage) return false
  return !!workflow?.transitions.some((tr) => tr.from_status_id === fromStatusId && tr.to_status_id === toStatus.id)
}

export function KanbanPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)
  const queryClient = useQueryClient()

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: workflow } = useQuery({
    queryKey: ['project', pid, 'workflow'],
    queryFn: () => getProjectWorkflow(pid),
  })
  const { data: tickets } = useQuery({ queryKey: ['tickets', pid, 'all'], queryFn: () => listAllProjectTickets(pid) })
  const { data: milestones } = useQuery({ queryKey: ['milestones', pid], queryFn: () => listMilestones(pid) })
  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })
  const { canManage } = useProjectRole(pid)

  const [milestoneFilter, setMilestoneFilter] = useState<number | 'all' | 'none'>('all')
  const [assigneeFilter, setAssigneeFilter] = useState<number | ''>('')
  const [overdueOnly, setOverdueOnly] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [draggingTicket, setDraggingTicket] = useState<Ticket | null>(null)

  // After a drag-and-drop, dnd-kit calls stopPropagation() (not preventDefault())
  // on the resulting click, on a listener it attaches directly to `document`
  // during the drag. That stops the click from ever reaching React's
  // delegated event system (rooted below `document`), which means a card's
  // own onClick handler never runs - but it does NOT cancel the click's
  // default action, so a nested <Link>'s native navigation still fires.
  // We track pointerdown/click position ourselves via listeners on the same
  // `document` node (stopPropagation doesn't stop other listeners on that
  // node from running) and call preventDefault() there when the pointer
  // moved far enough to have been a drag rather than a plain click.
  const pointerDownPos = useRef<{ x: number; y: number } | null>(null)

  useEffect(() => {
    function handlePointerDown(e: PointerEvent) {
      pointerDownPos.current = { x: e.clientX, y: e.clientY }
    }
    function handleClick(e: MouseEvent) {
      const start = pointerDownPos.current
      if (start && (Math.abs(e.clientX - start.x) > 5 || Math.abs(e.clientY - start.y) > 5)) {
        e.preventDefault()
      }
    }
    document.addEventListener('pointerdown', handlePointerDown, true)
    document.addEventListener('click', handleClick, true)
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown, true)
      document.removeEventListener('click', handleClick, true)
    }
  }, [])

  const moveMutation = useMutation({
    mutationFn: ({ ticket, statusId }: { ticket: Ticket; statusId: number }) =>
      updateTicket(ticket.id, { status_id: statusId, expected_updated_at: ticket.updated_at }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tickets', pid] })
      setError(null)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'kanban.moveFailed'))
      queryClient.invalidateQueries({ queryKey: ['tickets', pid] })
    },
  })

  function handleDragStart(event: DragStartEvent) {
    const ticket = tickets?.find((tkt) => tkt.id === Number(event.active.id))
    setDraggingTicket(ticket ?? null)
  }

  function handleDragEnd(event: DragEndEvent) {
    setDraggingTicket(null)
    const { active, over } = event
    if (!over || project?.archived_at) return
    const ticketId = Number(active.id)
    const statusId = Number(over.id)
    const fromTicket = draggingTicket?.id === ticketId ? draggingTicket : tickets?.find((tkt) => tkt.id === ticketId)
    const toStatus = statuses.find((s) => s.id === statusId)
    if (fromTicket && toStatus && !isReachable(workflow, fromTicket.status.id, toStatus, canManage)) {
      setError(t('kanban.invalidTransition'))
      return
    }
    if (!fromTicket || fromTicket.status.id === statusId) return
    moveMutation.mutate({ ticket: fromTicket, statusId })
  }

  const statuses = [...(workflow?.statuses ?? [])].sort((a, b) => a.position - b.position)

  const visibleTickets = (tickets ?? []).filter((ticket) => {
    if (milestoneFilter === 'none' && ticket.milestone_id !== null) return false
    if (typeof milestoneFilter === 'number' && ticket.milestone_id !== milestoneFilter) return false
    if (assigneeFilter !== '' && ticket.assignee_id !== assigneeFilter) return false
    if (overdueOnly && !isOverdue(ticket.due_date, ticket.status)) return false
    return true
  })

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('kanban.titleWithProject', { project: project.name }) : t('kanban.title')}
        </h1>
        <Link to="/projects" className="text-sm text-blue-600 hover:underline">
          {t('kanban.backToProjects')}
        </Link>
      </div>

      <ProjectNav project={project} />

      <div className="mb-4 flex flex-wrap items-center gap-3 rounded border bg-white p-3 text-sm">
        <label className="flex items-center gap-1">
          <span className="text-gray-600">{t('kanban.milestoneFilter')}</span>
          <select
            value={milestoneFilter}
            onChange={(e) => {
              const value = e.target.value
              setMilestoneFilter(value === 'all' || value === 'none' ? value : Number(value))
            }}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="all">{t('kanban.allTickets')}</option>
            {milestones?.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
            <option value="none">{t('kanban.noMilestone')}</option>
          </select>
        </label>

        <label className="flex items-center gap-1">
          <span className="text-gray-600">{t('ticket.list.assignee')}</span>
          <select
            value={assigneeFilter}
            onChange={(e) => setAssigneeFilter(e.target.value ? Number(e.target.value) : '')}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="">{t('ticket.list.all')}</option>
            {users?.map((u) => (
              <option key={u.id} value={u.id}>
                {u.display_name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-1">
          <input type="checkbox" checked={overdueOnly} onChange={(e) => setOverdueOnly(e.target.checked)} />
          <span className="text-gray-600">{t('ticket.list.overdueOnly')}</span>
        </label>
      </div>

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      <DndContext onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
        <div className="flex gap-3 overflow-x-auto pb-4">
          {statuses.map((status) => (
            <KanbanColumnDropzone
              key={status.id}
              status={status}
              tickets={visibleTickets.filter((t) => t.status.id === status.id)}
              invalidTarget={!!draggingTicket && !isReachable(workflow, draggingTicket.status.id, status, canManage)}
            />
          ))}
          {statuses.length === 0 && (
            <p className="text-sm text-gray-400">{t('kanban.empty')}</p>
          )}
        </div>
      </DndContext>
    </div>
  )
}
