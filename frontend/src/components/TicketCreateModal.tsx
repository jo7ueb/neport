import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { createTicket } from '../api/tickets'
import { listMilestones } from '../api/milestones'
import { useAssignableUsers } from '../lib/useAssignableUsers'
import { Modal } from './Modal'
import { MarkdownField } from './MarkdownField'
import { UserPicker } from './UserPicker'
import { getErrorMessage } from '../i18n/apiErrors'
import { STORY_POINT_OPTIONS } from '../lib/storyPoints'
import { StoryPointGuide } from './StoryPointGuide'
import type { User } from '../types/models'

interface TicketCreateModalProps {
  open: boolean
  onClose: () => void
  projectId: number
  parentTicketId?: number
  onCreated: () => void
}

export function TicketCreateModal({ open, onClose, projectId, parentTicketId, onCreated }: TicketCreateModalProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { data: milestones } = useQuery({
    queryKey: ['milestones', projectId],
    queryFn: () => listMilestones(projectId),
    enabled: open,
  })
  const assignableUsers = useAssignableUsers(projectId)

  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [storyPoints, setStoryPoints] = useState<number | ''>('')
  const [assignee, setAssignee] = useState<User | null>(null)
  const [dueDate, setDueDate] = useState('')
  const [milestoneId, setMilestoneId] = useState<number | ''>('')
  const [error, setError] = useState<string | null>(null)

  function resetForm() {
    setTitle('')
    setDescription('')
    setStoryPoints('')
    setAssignee(null)
    setDueDate('')
    setMilestoneId('')
    setError(null)
  }

  function handleClose() {
    resetForm()
    onClose()
  }

  const createMutation = useMutation({
    mutationFn: () =>
      createTicket({
        project_id: projectId,
        parent_ticket_id: parentTicketId,
        title,
        description: description || undefined,
        story_points: storyPoints === '' ? null : storyPoints,
        assignee_id: assignee?.id ?? undefined,
        due_date: dueDate || undefined,
        milestone_id: milestoneId === '' ? undefined : milestoneId,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project', projectId, 'ticket-summary'] })
      queryClient.invalidateQueries({ queryKey: ['milestones', projectId] })
      resetForm()
      onClose()
      onCreated()
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'ticket.list.createFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    createMutation.mutate()
  }

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title={t('ticket.list.createTitle')}
      className="max-h-[calc(100vh-2rem)] max-w-4xl overflow-y-auto"
    >
      <form onSubmit={handleSubmit} className="space-y-3">
        {error && <div className="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
        <input
          type="text"
          required
          autoFocus
          placeholder={t('ticket.list.newTitlePlaceholder')}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          className="w-full rounded border border-gray-300 px-3 py-2"
        />

        <div>
          <label className="mb-1 block text-sm font-medium text-gray-700">{t('ticket.detail.descriptionLabel')}</label>
          <MarkdownField value={description} onChange={setDescription} rows={5} />
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <div className="flex-1">
            <label className="mb-1 block text-sm font-medium text-gray-700">{t('ticket.detail.assigneeLabel')}</label>
            <UserPicker value={assignee} onChange={setAssignee} candidates={assignableUsers ?? []} />
          </div>
          <div className="flex-1">
            <label className="mb-1 block text-sm font-medium text-gray-700">
              <StoryPointGuide />
            </label>
            <select
              value={storyPoints}
              onChange={(e) => setStoryPoints(e.target.value ? Number(e.target.value) : '')}
              className="w-full rounded border border-gray-300 px-2 py-1.5 text-sm"
            >
              <option value="">{t('ticket.detail.storyPointsUnset')}</option>
              {STORY_POINT_OPTIONS.map((point) => (
                <option key={point} value={point}>
                  {point}
                </option>
              ))}
            </select>
          </div>
          <div className="flex-1">
            <label className="mb-1 block text-sm font-medium text-gray-700">{t('ticket.detail.dueDateLabel')}</label>
            <input
              type="date"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1.5 text-sm"
            />
          </div>
        </div>

        <div>
          <label className="mb-1 block text-sm font-medium text-gray-700">{t('ticket.detail.milestoneLabel')}</label>
          <select
            value={milestoneId}
            onChange={(e) => setMilestoneId(e.target.value ? Number(e.target.value) : '')}
            className="w-full rounded border border-gray-300 px-2 py-1.5 text-sm"
          >
            <option value="">{t('ticket.detail.noMilestone')}</option>
            {milestones?.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </select>
        </div>

        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={handleClose}
            className="rounded border border-gray-300 px-4 py-2 text-gray-600 hover:bg-gray-50"
          >
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={createMutation.isPending}
            className="rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {t('common.create')}
          </button>
        </div>
      </form>
    </Modal>
  )
}
