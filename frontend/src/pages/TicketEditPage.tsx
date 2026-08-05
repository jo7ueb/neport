import { useEffect, useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getTicket, updateTicket } from '../api/tickets'
import { useAssignableUsers } from '../lib/useAssignableUsers'
import { useProjectRole } from '../lib/useProjectRole'
import { listMilestones } from '../api/milestones'
import { getProjectWorkflow } from '../api/projects'
import { MarkdownField } from '../components/MarkdownField'
import { TicketPicker } from '../components/TicketPicker'
import { getErrorMessage } from '../i18n/apiErrors'
import { statusLabel } from '../i18n/statusLabel'
import { STORY_POINT_OPTIONS } from '../lib/storyPoints'
import { StoryPointGuide } from '../components/StoryPointGuide'
import type { Ticket, WorkflowStatus } from '../types/models'

export function TicketEditPage() {
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

  const users = useAssignableUsers(ticket?.project_id)
  const { canManage } = useProjectRole(ticket?.project_id)
  const { data: milestones } = useQuery({
    queryKey: ['milestones', ticket?.project_id],
    queryFn: () => listMilestones(ticket!.project_id),
    enabled: !!ticket,
  })
  const { data: workflow } = useQuery({
    queryKey: ['project', ticket?.project_id, 'workflow'],
    queryFn: () => getProjectWorkflow(ticket!.project_id),
    enabled: !!ticket,
  })
  const { data: currentParentTicket } = useQuery({
    queryKey: ['ticket', ticket?.parent_ticket_id, 'as-parent-of', tid],
    queryFn: () => getTicket(ticket!.parent_ticket_id!),
    enabled: !!ticket?.parent_ticket_id,
  })

  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [storyPoints, setStoryPoints] = useState<number | ''>('')
  const [statusId, setStatusId] = useState<number | ''>('')
  const [assigneeId, setAssigneeId] = useState<number | ''>('')
  const [dueDate, setDueDate] = useState('')
  const [parentTicket, setParentTicket] = useState<Ticket | null>(null)
  const [milestoneId, setMilestoneId] = useState<number | ''>('')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (ticket) {
      setTitle(ticket.title)
      setDescription(ticket.description ?? '')
      setStoryPoints(ticket.child_ticket_count > 0 ? '' : ticket.story_points ?? '')
      setStatusId(ticket.status.id)
      setAssigneeId(ticket.assignee_id ?? '')
      setDueDate(ticket.due_date ?? '')
      setMilestoneId(ticket.milestone_id ?? '')
      if (!ticket.parent_ticket_id) {
        setParentTicket(null)
      }
    }
  }, [ticket])

  // The parent ticket's own title/id only become available once this
  // separate lookup resolves, so it's synced independently from the rest of
  // the form fields above.
  useEffect(() => {
    if (ticket && currentParentTicket && currentParentTicket.id === ticket.parent_ticket_id) {
      setParentTicket(currentParentTicket)
    }
  }, [ticket, currentParentTicket])

  const updateMutation = useMutation({
    mutationFn: () =>
      updateTicket(tid, {
        title,
        description: description || null,
        story_points: storyPoints === '' ? null : storyPoints,
        status_id: statusId === '' ? undefined : statusId,
        assignee_id: assigneeId === '' ? null : assigneeId,
        due_date: dueDate || null,
        parent_ticket_id: parentTicket ? parentTicket.id : null,
        milestone_id: milestoneId === '' ? null : milestoneId,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ticket', tid] })
      queryClient.invalidateQueries({ queryKey: ['project', ticket?.project_id, 'ticket-summary'] })
      queryClient.invalidateQueries({ queryKey: ['milestones', ticket?.project_id] })
      navigate(`/tickets/${tid}`)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'ticket.detail.updateFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    updateMutation.mutate()
  }

  if (isError) {
    return <p className="text-sm text-red-600">{t('ticket.detail.notFound', { id: tid })}</p>
  }
  if (isLoading || !ticket) {
    return <p className="text-gray-500">{t('common.loading')}</p>
  }

  // Only offer the current status plus whatever the workflow's transitions
  // allow moving to from here - picking anything else would just bounce
  // back as a 400 from the backend's transition check.
  const reachableStatusIds = new Set(
    workflow?.transitions
      .filter((tr) => tr.from_status_id === ticket.status.id)
      .map((tr) => tr.to_status_id)
  )
  // Moving into a "confirmed"-category status (closing a done ticket after
  // review) is a project manager's call - offering it to anyone else would
  // just bounce back as a 403 from the backend (see access::Access::Manage).
  // The ticket's current status always stays in the list even if it's
  // already confirmed and the viewer isn't a manager, so the dropdown still
  // has a value to show.
  const statusOptions: WorkflowStatus[] =
    workflow?.statuses.filter(
      (s) => s.id === ticket.status.id || (reachableStatusIds.has(s.id) && (s.category !== 'confirmed' || canManage))
    ) ?? [ticket.status]

  return (
    <div>
      <Link to={`/tickets/${tid}`} className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('ticket.detail.backToTicket')}
      </Link>

      <h1 className="mb-4 text-2xl font-semibold text-gray-800">
        {t('ticket.detail.editTitle', { id: ticket.id })}
      </h1>

      <form onSubmit={handleSubmit} className="rounded border bg-white p-4">
        {error && <div className="mb-3 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

        <div className="grid gap-3 sm:grid-cols-2">
          <label className="text-sm sm:col-span-2">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.titleLabel')}</span>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="w-full rounded border border-gray-300 px-3 py-2"
            />
          </label>

          <label className="text-sm sm:col-span-2">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.descriptionLabel')}</span>
            <MarkdownField
              value={description}
              onChange={setDescription}
              rows={8}
              startInPreview={!!ticket.description}
            />
          </label>

          <label className="text-sm">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.statusLabel')}</span>
            <select
              value={statusId}
              onChange={(e) => setStatusId(Number(e.target.value))}
              className="w-full rounded border border-gray-300 px-3 py-2"
            >
              {statusOptions.map((s) => (
                <option key={s.id} value={s.id}>
                  {statusLabel(t, s)}
                </option>
              ))}
            </select>
          </label>

          <label className="text-sm">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.assigneeLabel')}</span>
            <select
              value={assigneeId}
              onChange={(e) => setAssigneeId(e.target.value ? Number(e.target.value) : '')}
              className="w-full rounded border border-gray-300 px-3 py-2"
            >
              <option value="">{t('common.unassigned')}</option>
              {users?.map((u) => (
                <option key={u.id} value={u.id}>
                  {u.display_name}
                </option>
              ))}
            </select>
          </label>

          <label className="text-sm">
            <span className="mb-1 block text-gray-600">
              <StoryPointGuide />
            </span>
            {ticket.child_ticket_count > 0 ? (
              <div className="rounded border border-gray-200 bg-gray-50 px-3 py-2 text-gray-500">
                {t('ticket.detail.storyPointsNotApplicable')}
              </div>
            ) : (
              <select
                value={storyPoints}
                onChange={(e) => setStoryPoints(e.target.value ? Number(e.target.value) : '')}
                className="w-full rounded border border-gray-300 px-3 py-2"
              >
                <option value="">{t('ticket.detail.storyPointsUnset')}</option>
                {STORY_POINT_OPTIONS.map((point) => (
                  <option key={point} value={point}>
                    {point}
                  </option>
                ))}
              </select>
            )}
          </label>

          <label className="text-sm">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.dueDateLabel')}</span>
            <input
              type="date"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
              className="w-full rounded border border-gray-300 px-3 py-2"
            />
          </label>

          <label className="text-sm">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.parentTicketLabel')}</span>
            <TicketPicker value={parentTicket} onChange={setParentTicket} excludeId={tid} />
          </label>

          <label className="text-sm">
            <span className="mb-1 block text-gray-600">{t('ticket.detail.milestoneLabel')}</span>
            <select
              value={milestoneId}
              onChange={(e) => setMilestoneId(e.target.value ? Number(e.target.value) : '')}
              className="w-full rounded border border-gray-300 px-3 py-2"
            >
              <option value="">{t('ticket.detail.noMilestone')}</option>
              {milestones?.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="mt-4 flex justify-end gap-2">
          <Link
            to={`/tickets/${tid}`}
            className="rounded border border-gray-300 px-4 py-2 text-sm text-gray-600 hover:bg-gray-50"
          >
            {t('common.cancel')}
          </Link>
          <button
            type="submit"
            disabled={updateMutation.isPending}
            className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {t('ticket.detail.save')}
          </button>
        </div>
      </form>
    </div>
  )
}
