import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { DndContext, useDraggable, useDroppable, type DragEndEvent, type DragStartEvent } from '@dnd-kit/core'
import { getProject, getProjectWorkflow } from '../api/projects'
import { listMilestones } from '../api/milestones'
import { listAllProjectTickets, updateTicket } from '../api/tickets'
import { ProjectNav } from '../components/ProjectNav'
import { getErrorMessage } from '../i18n/apiErrors'
import { statusLabel } from '../i18n/statusLabel'
import type { Ticket, WorkflowDetail, WorkflowStatus } from '../types/models'

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
// between them.
function isReachable(workflow: WorkflowDetail | undefined, fromStatusId: number, toStatusId: number): boolean {
  if (fromStatusId === toStatusId) return true
  return !!workflow?.transitions.some((tr) => tr.from_status_id === fromStatusId && tr.to_status_id === toStatusId)
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

  const [milestoneFilter, setMilestoneFilter] = useState<number | 'all' | 'none'>('all')
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
    mutationFn: ({ ticketId, statusId }: { ticketId: number; statusId: number }) =>
      updateTicket(ticketId, { status_id: statusId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tickets', pid] })
      setError(null)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'kanban.moveFailed'))
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
    const fromTicket = tickets?.find((tkt) => tkt.id === ticketId)
    if (fromTicket && !isReachable(workflow, fromTicket.status.id, statusId)) {
      setError(t('kanban.invalidTransition'))
      return
    }
    if (fromTicket && fromTicket.status.id === statusId) return
    moveMutation.mutate({ ticketId, statusId })
  }

  const statuses = [...(workflow?.statuses ?? [])].sort((a, b) => a.position - b.position)

  const visibleTickets = (tickets ?? []).filter((ticket) => {
    if (milestoneFilter === 'all') return true
    if (milestoneFilter === 'none') return ticket.milestone_id === null
    return ticket.milestone_id === milestoneFilter
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

      <ProjectNav projectId={pid} archived={!!project?.archived_at} />

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
      </div>

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      <DndContext onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
        <div className="flex gap-3 overflow-x-auto pb-4">
          {statuses.map((status) => (
            <KanbanColumnDropzone
              key={status.id}
              status={status}
              tickets={visibleTickets.filter((t) => t.status.id === status.id)}
              invalidTarget={!!draggingTicket && !isReachable(workflow, draggingTicket.status.id, status.id)}
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
