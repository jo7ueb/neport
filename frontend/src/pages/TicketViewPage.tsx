import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { deleteTicket, getActualDuration, getTicket, listTicketChildren, updateTicket } from '../api/tickets'
import { listAllUsers } from '../api/users'
import { listMilestones } from '../api/milestones'
import { getProjectWorkflow } from '../api/projects'
import { DependencyList } from '../components/DependencyList'
import { MarkdownContent } from '../components/MarkdownContent'
import { TicketCreateModal } from '../components/TicketCreateModal'
import { TicketPicker } from '../components/TicketPicker'
import { TicketProgress } from '../components/TicketProgress'
import { TicketRow } from '../components/TicketRow'
import { TicketTimeline } from '../components/TicketTimeline'
import { getErrorMessage } from '../i18n/apiErrors'
import { statusLabel } from '../i18n/statusLabel'
import type { Ticket } from '../types/models'

export function TicketViewPage() {
  const { t } = useTranslation()
  const { ticketId } = useParams()
  const tid = Number(ticketId)
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const { data: ticket, isLoading, isError } = useQuery({
    queryKey: ['ticket', tid],
    queryFn: () => getTicket(tid),
    retry: false,
  })

  const { data: children } = useQuery({
    queryKey: ['ticket', tid, 'children'],
    queryFn: () => listTicketChildren(tid),
    enabled: !!ticket,
  })

  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })
  const { data: milestones } = useQuery({
    queryKey: ['milestones', ticket?.project_id],
    queryFn: () => listMilestones(ticket!.project_id),
    enabled: !!ticket,
  })
  const { data: parentTicket } = useQuery({
    queryKey: ['ticket', ticket?.parent_ticket_id, 'as-parent-of', tid],
    queryFn: () => getTicket(ticket!.parent_ticket_id!),
    enabled: !!ticket?.parent_ticket_id,
  })
  const { data: actualDuration } = useQuery({
    queryKey: ['ticket', tid, 'actual-duration'],
    queryFn: () => getActualDuration(tid),
    enabled: !!ticket,
  })
  const { data: workflow } = useQuery({
    queryKey: ['project', ticket?.project_id, 'workflow'],
    queryFn: () => getProjectWorkflow(ticket!.project_id),
    enabled: !!ticket,
  })

  const [error, setError] = useState<string | null>(null)
  const [showCreateChild, setShowCreateChild] = useState(false)
  const [existingChildTicket, setExistingChildTicket] = useState<Ticket | null>(null)

  const deleteMutation = useMutation({
    mutationFn: () => deleteTicket(tid),
    onSuccess: () => navigate(`/projects/${ticket!.project_id}/tickets`),
    onError: (err) => {
      setError(getErrorMessage(err, t, 'ticket.detail.deleteFailed'))
    },
  })

  function handleDelete() {
    const message = children?.length
      ? t('ticket.detail.confirmDeleteWithChildren', { count: children.length })
      : t('ticket.detail.confirmDelete')
    if (window.confirm(message)) {
      deleteMutation.mutate()
    }
  }

  const attachChildMutation = useMutation({
    mutationFn: () => updateTicket(existingChildTicket!.id, { parent_ticket_id: tid }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ticket', tid, 'children'] })
      setExistingChildTicket(null)
      setError(null)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'ticket.detail.addChildFailed'))
    },
  })

  if (isError) {
    return <p className="text-sm text-red-600">{t('ticket.detail.notFound', { id: tid })}</p>
  }
  if (isLoading || !ticket) {
    return <p className="text-gray-500">{t('common.loading')}</p>
  }

  function assigneeName(id: number | null): string {
    if (id === null) return t('common.unassigned')
    return users?.find((u) => u.id === id)?.display_name ?? `#${id}`
  }

  function milestoneName(id: number | null): string | null {
    if (id === null) return null
    return milestones?.find((m) => m.id === id)?.name ?? `#${id}`
  }

  function formatActualDuration(seconds: number): string {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    return t('ticket.detail.actualDurationValue', { hours, minutes })
  }

  return (
    <div>
      <Link
        to={`/projects/${ticket.project_id}/tickets`}
        className="mb-4 inline-block text-sm text-blue-600 hover:underline"
      >
        {t('ticket.detail.backToList')}
      </Link>

      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          #{ticket.id} {ticket.title}
        </h1>
        <div className="flex gap-2">
          <Link
            to={`/tickets/${ticket.id}/edit`}
            className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
          >
            {t('ticket.detail.edit')}
          </Link>
          <button
            onClick={handleDelete}
            disabled={deleteMutation.isPending}
            className="rounded border border-red-300 px-3 py-1.5 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
          >
            {t('ticket.detail.delete')}
          </button>
        </div>
      </div>

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="space-y-6 lg:col-span-2">
          <div className="rounded border bg-white p-4">
            <div className="mb-1 text-sm text-gray-500">{t('ticket.detail.descriptionLabel')}</div>
            {ticket.description ? (
              <MarkdownContent text={ticket.description} />
            ) : (
              <p className="text-sm text-gray-400">{t('ticket.detail.noDescription')}</p>
            )}
          </div>

          <div className="rounded border bg-white p-4">
            <TicketTimeline ticketId={tid} statuses={workflow?.statuses ?? []} />
          </div>
        </div>

        <div className="space-y-6">
          <div className="rounded border bg-white p-4">
            <dl className="grid gap-3 text-sm">
              <div>
                <dt className="text-gray-500">{t('ticket.detail.statusLabel')}</dt>
                <dd className="text-gray-800">{statusLabel(t, ticket.status)}</dd>
              </div>
              {ticket.child_ticket_count > 0 && (
                <div>
                  <dt className="text-gray-500">{t('ticket.detail.childProgressLabel')}</dt>
                  <dd className="text-gray-800">
                    <TicketProgress done={ticket.child_done_ticket_count} total={ticket.child_ticket_count} />
                  </dd>
                </div>
              )}
              <div>
                <dt className="text-gray-500">{t('ticket.detail.assigneeLabel')}</dt>
                <dd className="text-gray-800">{assigneeName(ticket.assignee_id)}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('ticket.detail.actualDurationLabel')}</dt>
                <dd className="text-gray-800">
                  {actualDuration ? formatActualDuration(actualDuration.in_progress_seconds) : '-'}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('ticket.detail.dueDateLabel')}</dt>
                <dd className="text-gray-800">{ticket.due_date ?? '-'}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('ticket.detail.milestoneLabel')}</dt>
                <dd className="text-gray-800">{milestoneName(ticket.milestone_id) ?? '-'}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('ticket.detail.parentTicket')}</dt>
                <dd className="text-gray-800">
                  {parentTicket ? (
                    <Link to={`/tickets/${parentTicket.id}`} className="text-blue-600 hover:underline">
                      #{parentTicket.id} {parentTicket.title}
                    </Link>
                  ) : (
                    '-'
                  )}
                </dd>
              </div>
            </dl>
          </div>

          <div className="rounded border bg-white p-4">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="text-lg font-semibold text-gray-800">{t('ticket.detail.children')}</h2>
              <button
                onClick={() => setShowCreateChild(true)}
                className="rounded bg-blue-600 px-3 py-1 text-sm text-white hover:bg-blue-700"
              >
                {t('common.create')}
              </button>
            </div>
            <div className="mb-3 flex items-center gap-2">
              <TicketPicker
                value={existingChildTicket}
                onChange={setExistingChildTicket}
                placeholder={t('ticket.detail.existingChildPlaceholder')}
                excludeId={tid}
                className="flex-1"
              />
              <button
                onClick={() => attachChildMutation.mutate()}
                disabled={!existingChildTicket || attachChildMutation.isPending}
                className="rounded bg-blue-600 px-3 py-1 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {t('ticket.detail.add')}
              </button>
            </div>
            <div className="space-y-1">
              {children?.map((child) => (
                <TicketRow key={child.id} ticket={child} />
              ))}
              {children?.length === 0 && <p className="text-sm text-gray-400">{t('ticket.detail.noChildren')}</p>}
            </div>
          </div>

          <div className="rounded border bg-white p-4">
            <DependencyList ticketId={tid} />
          </div>
        </div>
      </div>

      <TicketCreateModal
        open={showCreateChild}
        onClose={() => setShowCreateChild(false)}
        projectId={ticket.project_id}
        parentTicketId={tid}
        onCreated={() => queryClient.invalidateQueries({ queryKey: ['ticket', tid, 'children'] })}
      />
    </div>
  )
}
