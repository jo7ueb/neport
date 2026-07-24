import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject } from '../api/projects'
import { createMilestone, deleteMilestone, listMilestones, updateMilestone, type Milestone } from '../api/milestones'
import { listAllProjectTickets } from '../api/tickets'
import { ProjectNav } from '../components/ProjectNav'
import { Modal } from '../components/Modal'
import { getErrorMessage } from '../i18n/apiErrors'
import type { Ticket } from '../types/models'

function daysUntil(dueDate: string): number {
  const today = new Date().toISOString().slice(0, 10)
  const msPerDay = 24 * 60 * 60 * 1000
  return Math.round((Date.parse(dueDate) - Date.parse(today)) / msPerDay)
}

function MilestoneProgress({ milestone, tickets }: { milestone: Milestone; tickets: Ticket[] }) {
  const { t } = useTranslation()
  const ticketsForMilestone = tickets.filter((tkt) => tkt.milestone_id === milestone.id)
  const total = ticketsForMilestone.length
  const done = ticketsForMilestone.filter(
    (tkt) => tkt.status.category === 'done' || tkt.status.category === 'confirmed'
  ).length
  const percent = total === 0 ? 0 : Math.round((done / total) * 100)

  let dueText: { key: 'milestone.dueToday' | 'milestone.daysRemaining' | 'milestone.overdueDays'; days: number } | null = null
  if (milestone.due_date) {
    const days = daysUntil(milestone.due_date)
    if (days === 0) dueText = { key: 'milestone.dueToday', days }
    else if (days > 0) dueText = { key: 'milestone.daysRemaining', days }
    else dueText = { key: 'milestone.overdueDays', days: Math.abs(days) }
  }

  return (
    <div className="mt-1 flex items-center gap-3 text-sm">
      {total === 0 ? (
        <span className="text-gray-400">{t('milestone.noTickets')}</span>
      ) : (
        <>
          <div className="h-2 w-32 overflow-hidden rounded bg-gray-200">
            <div className="h-full bg-blue-600" style={{ width: `${percent}%` }} />
          </div>
          <span className="text-gray-600">{t('milestone.progress', { done, total })}</span>
        </>
      )}
      {dueText && (
        <span className={dueText.key === 'milestone.overdueDays' ? 'font-semibold text-red-600' : 'text-gray-500'}>
          {t(dueText.key, { days: dueText.days })}
        </span>
      )}
    </div>
  )
}

function MilestoneEditForm({
  milestone,
  onSave,
  onCancel,
  isSaving,
}: {
  milestone: Milestone
  onSave: (fields: { name: string; start_date: string; due_date: string; description: string }) => void
  onCancel: () => void
  isSaving: boolean
}) {
  const { t } = useTranslation()
  const [name, setName] = useState(milestone.name)
  const [startDate, setStartDate] = useState(milestone.start_date ?? '')
  const [dueDate, setDueDate] = useState(milestone.due_date ?? '')
  const [description, setDescription] = useState(milestone.description ?? '')

  return (
    <div className="flex flex-1 flex-wrap items-end gap-2 text-sm">
      <label>
        <span className="mb-1 block text-gray-600">{t('milestone.name')}</span>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1"
        />
      </label>
      <label>
        <span className="mb-1 block text-gray-600">{t('milestone.startDate')}</span>
        <input
          type="date"
          value={startDate}
          onChange={(e) => setStartDate(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1"
        />
      </label>
      <label>
        <span className="mb-1 block text-gray-600">{t('milestone.dueDate')}</span>
        <input
          type="date"
          value={dueDate}
          onChange={(e) => setDueDate(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1"
        />
      </label>
      <label className="flex-1">
        <span className="mb-1 block text-gray-600">{t('milestone.description')}</span>
        <input
          type="text"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          className="w-full rounded border border-gray-300 px-2 py-1"
        />
      </label>
      <button
        onClick={() => name.trim() && onSave({ name: name.trim(), start_date: startDate, due_date: dueDate, description })}
        disabled={isSaving || !name.trim()}
        className="rounded bg-blue-600 px-3 py-1 text-white hover:bg-blue-700 disabled:opacity-50"
      >
        {t('milestone.save')}
      </button>
      <button onClick={onCancel} className="rounded border border-gray-300 px-3 py-1 text-gray-600 hover:bg-gray-50">
        {t('milestone.cancel')}
      </button>
    </div>
  )
}

export function MilestonesPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)
  const queryClient = useQueryClient()

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: milestones, isLoading } = useQuery({
    queryKey: ['milestones', pid],
    queryFn: () => listMilestones(pid),
  })
  const { data: tickets } = useQuery({
    queryKey: ['tickets', pid, 'all'],
    queryFn: () => listAllProjectTickets(pid),
  })

  const [showCreate, setShowCreate] = useState(false)
  const [name, setName] = useState('')
  const [startDate, setStartDate] = useState('')
  const [dueDate, setDueDate] = useState('')
  const [description, setDescription] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [editingId, setEditingId] = useState<number | null>(null)

  const createMutation = useMutation({
    mutationFn: () => createMilestone(pid, name, dueDate || undefined, description || undefined, startDate || undefined),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['milestones', pid] })
      setName('')
      setStartDate('')
      setDueDate('')
      setDescription('')
      setError(null)
      setShowCreate(false)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'milestone.createFailed'))
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (milestoneId: number) => deleteMilestone(pid, milestoneId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['milestones', pid] }),
  })

  const updateMutation = useMutation({
    mutationFn: ({
      milestoneId,
      fields,
    }: {
      milestoneId: number
      fields: { name: string; start_date: string; due_date: string; description: string }
    }) =>
      // The backend can only leave start_date/due_date/description unchanged
      // when the field is omitted (it can't yet clear them to null via this
      // endpoint), so an empty input must be dropped rather than sent as "".
      updateMilestone(pid, milestoneId, {
        name: fields.name,
        start_date: fields.start_date || undefined,
        due_date: fields.due_date || undefined,
        description: fields.description || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['milestones', pid] })
      setEditingId(null)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'milestone.updateFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    createMutation.mutate()
  }

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('milestone.titleWithProject', { project: project.name }) : t('milestone.title')}
        </h1>
        <div className="flex items-center gap-4">
          {!project?.archived_at && (
            <button
              onClick={() => setShowCreate(true)}
              className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
            >
              {t('common.new')}
            </button>
          )}
          <Link to="/projects" className="text-sm text-blue-600 hover:underline">
            {t('milestone.backToProjects')}
          </Link>
        </div>
      </div>

      <ProjectNav project={project} />

      <Modal open={showCreate} onClose={() => setShowCreate(false)} title={t('milestone.title')}>
        <form onSubmit={handleSubmit} className="space-y-3 text-sm">
          {error && <div className="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('milestone.name')}</span>
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('milestone.startDate')}</span>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('milestone.dueDate')}</span>
            <input
              type="date"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('milestone.description')}</span>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowCreate(false)}
              className="rounded border border-gray-300 px-3 py-1.5 text-gray-600 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={createMutation.isPending}
              className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('milestone.create')}
            </button>
          </div>
        </form>
      </Modal>

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <ul className="divide-y rounded border bg-white">
          {milestones?.map((m) =>
            editingId === m.id ? (
              <li key={m.id} className="flex items-center px-4 py-3">
                <MilestoneEditForm
                  milestone={m}
                  isSaving={updateMutation.isPending}
                  onCancel={() => setEditingId(null)}
                  onSave={(fields) => updateMutation.mutate({ milestoneId: m.id, fields })}
                />
              </li>
            ) : (
              <li key={m.id} className="flex items-center justify-between px-4 py-3">
                <div>
                  <div className="font-medium text-gray-800">{m.name}</div>
                  {m.start_date && <div className="text-sm text-gray-500">{t('milestone.startsOn', { date: m.start_date })}</div>}
                  {m.due_date && <div className="text-sm text-gray-500">{t('milestone.due', { date: m.due_date })}</div>}
                  {m.description && <div className="text-sm text-gray-500">{m.description}</div>}
                  <MilestoneProgress milestone={m} tickets={tickets ?? []} />
                </div>
                <div className="space-x-3">
                  <Link to={`/projects/${pid}/milestones/${m.id}/burndown`} className="text-sm text-blue-600 hover:underline">
                    {t('milestone.burndown')}
                  </Link>
                  <button
                    onClick={() => setEditingId(m.id)}
                    className="text-sm text-blue-600 hover:underline"
                  >
                    {t('milestone.edit')}
                  </button>
                  <button
                    onClick={() => deleteMutation.mutate(m.id)}
                    className="text-sm text-red-600 hover:underline"
                  >
                    {t('milestone.delete')}
                  </button>
                </div>
              </li>
            )
          )}
          {milestones?.length === 0 && (
            <li className="px-4 py-6 text-center text-gray-400">{t('milestone.empty')}</li>
          )}
        </ul>
      )}
    </div>
  )
}
