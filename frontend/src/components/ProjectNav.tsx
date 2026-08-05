import { useState, type FormEvent } from 'react'
import { NavLink } from 'react-router-dom'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { updateProject } from '../api/projects'
import { useProjectRole } from '../lib/useProjectRole'
import { getErrorMessage } from '../i18n/apiErrors'
import { Modal } from './Modal'
import { formatEstimatedHours } from '../lib/storyPoints'
import type { Project } from '../types/models'

export function ProjectNav({ project }: { project: Project | undefined }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const projectId = project?.id
  const archived = !!project?.archived_at
  const { canManage } = useProjectRole(projectId)

  const [showEdit, setShowEdit] = useState(false)
  const [editName, setEditName] = useState('')
  const [editDescription, setEditDescription] = useState('')
  const [editHoursPerPoint, setEditHoursPerPoint] = useState('1')
  const [editError, setEditError] = useState<string | null>(null)

  function openEdit() {
    setEditName(project?.name ?? '')
    setEditDescription(project?.description ?? '')
    setEditHoursPerPoint(String(project?.hours_per_story_point ?? 1))
    setEditError(null)
    setShowEdit(true)
  }

  const updateProjectMutation = useMutation({
    mutationFn: () =>
      updateProject(projectId!, {
        name: editName,
        description: editDescription,
        hours_per_story_point: Number(editHoursPerPoint),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project', projectId] })
      queryClient.invalidateQueries({ queryKey: ['projects'] })
      queryClient.invalidateQueries({ queryKey: ['project', projectId, 'ticket-summary'] })
      queryClient.invalidateQueries({ queryKey: ['milestones', projectId] })
      setShowEdit(false)
    },
    onError: (err) => setEditError(getErrorMessage(err, t, 'projectMembers.editFailed')),
  })

  function handleEditSubmit(e: FormEvent) {
    e.preventDefault()
    if (!Number.isFinite(Number(editHoursPerPoint)) || Number(editHoursPerPoint) <= 0) {
      setEditError(t('projectMembers.invalidHoursPerPoint'))
      return
    }
    updateProjectMutation.mutate()
  }

  const velocityDisplay = project ? formatEstimatedHours(project.hours_per_story_point) : { hours: 1, minutes: 0 }

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `rounded px-3 py-1.5 text-sm ${isActive ? 'bg-blue-600 text-white' : 'text-gray-600 hover:bg-gray-100'}`

  return (
    <>
      <nav className="mb-4 flex items-center justify-between gap-2 rounded border bg-white p-2">
        <div className="flex flex-wrap gap-2">
          <NavLink to={`/projects/${projectId}/dashboard`} className={linkClass}>
            {t('projectNav.dashboard')}
          </NavLink>
          <NavLink to={`/projects/${projectId}/wbs`} className={linkClass}>
            {t('projectNav.wbs')}
          </NavLink>
          <NavLink to={`/projects/${projectId}/tickets`} className={linkClass} end>
            {t('projectNav.tickets')}
          </NavLink>
          <NavLink to={`/projects/${projectId}/kanban`} className={linkClass}>
            {t('projectNav.kanban')}
          </NavLink>
          <NavLink to={`/projects/${projectId}/milestones`} className={linkClass}>
            {t('projectNav.milestones')}
          </NavLink>
          <NavLink to={`/projects/${projectId}/members`} className={linkClass}>
            {t('projectNav.members')}
          </NavLink>
        </div>
        <div className="flex items-center gap-4">
          {project && (
            <span className="text-xs text-gray-500">
              {t('projectMembers.hoursPerPointSummary', {
                hours: velocityDisplay.hours,
                minutes: velocityDisplay.minutes,
              })}
            </span>
          )}
          {canManage && !archived && (
            <button onClick={openEdit} className="px-2 text-sm text-blue-600 hover:underline">
              {t('projectMembers.editProject')}
            </button>
          )}
        </div>
      </nav>
      {archived && (
        <div className="mb-4 rounded border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-800">
          {t('projectNav.archivedBanner')}
        </div>
      )}

      <Modal open={showEdit} onClose={() => setShowEdit(false)} title={t('projectMembers.editProject')}>
        <form onSubmit={handleEditSubmit} className="space-y-4">
          {editError && <div className="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{editError}</div>}
          <input
            type="text"
            required
            placeholder={t('project.list.namePlaceholder')}
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            className="w-full rounded border border-gray-300 px-3 py-2"
          />
          <input
            type="text"
            placeholder={t('project.list.descriptionPlaceholder')}
            value={editDescription}
            onChange={(e) => setEditDescription(e.target.value)}
            className="w-full rounded border border-gray-300 px-3 py-2"
          />
          <label className="block text-sm">
            <span className="mb-1 block text-gray-700">{t('projectMembers.hoursPerPointLabel')}</span>
            <input
              type="number"
              min="0.1"
              step="0.1"
              value={editHoursPerPoint}
              onChange={(e) => setEditHoursPerPoint(e.target.value)}
              className="w-full rounded border border-gray-300 px-3 py-2"
            />
            <span className="mt-1 block text-xs text-gray-500">{t('projectMembers.hoursPerPointHelp')}</span>
          </label>
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowEdit(false)}
              className="rounded border border-gray-300 px-4 py-2 text-gray-600 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={updateProjectMutation.isPending}
              className="rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('common.save')}
            </button>
          </div>
        </form>
      </Modal>
    </>
  )
}
