import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { archiveProject, createProject, listProjects, unarchiveProject, type InitialMemberInput } from '../api/projects'
import { listWorkflows } from '../api/workflows'
import { Pagination } from '../components/Pagination'
import { Modal } from '../components/Modal'
import { UserPicker } from '../components/UserPicker'
import { getErrorMessage } from '../i18n/apiErrors'
import { useAuth } from '../context/AuthContext'
import type { ProjectRole, User } from '../types/models'

const EXTRA_ROLE_OPTIONS: ProjectRole[] = ['member', 'viewer']

interface ExtraMemberRow {
  user: User | null
  role: ProjectRole
}

export function ProjectListPage() {
  const { t } = useTranslation()
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useState(25)
  const [scope, setScope] = useState<'mine' | 'all'>('mine')
  const [archivedTab, setArchivedTab] = useState(false)
  const isAdmin = user?.role === 'admin'
  const { data, isLoading } = useQuery({
    queryKey: ['projects', page, perPage, isAdmin ? scope : 'mine', archivedTab],
    queryFn: () => listProjects({ page, per_page: perPage, scope: isAdmin ? scope : undefined, archived: archivedTab }),
  })
  const projects = data?.items

  const archiveMutation = useMutation({
    mutationFn: archiveProject,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['projects'] }),
  })
  const unarchiveMutation = useMutation({
    mutationFn: unarchiveProject,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['projects'] }),
  })

  function handleArchive(id: number) {
    if (window.confirm(t('project.list.archiveConfirm'))) {
      archiveMutation.mutate(id)
    }
  }

  const { data: workflows } = useQuery({ queryKey: ['workflows'], queryFn: listWorkflows })
  const defaultWorkflowId = workflows?.find((w) => w.is_default)?.id

  const [showCreate, setShowCreate] = useState(false)
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [workflowId, setWorkflowId] = useState<number | ''>('')
  const [manager, setManager] = useState<User | null>(null)
  const [extraMembers, setExtraMembers] = useState<ExtraMemberRow[]>([])
  const [error, setError] = useState<string | null>(null)

  const usedUserIds = [manager?.id, ...extraMembers.map((m) => m.user?.id)].filter(
    (id): id is number => id !== undefined
  )
  const excludeIdsFor = (currentUserId?: number) => usedUserIds.filter((id) => id !== currentUserId)

  function resetForm() {
    setName('')
    setDescription('')
    setWorkflowId('')
    setManager(null)
    setExtraMembers([])
    setError(null)
  }

  const createMutation = useMutation({
    mutationFn: () => {
      const members: InitialMemberInput[] = [
        { user_id: manager!.id, role: 'manager' },
        ...extraMembers
          .filter((m): m is { user: User; role: ProjectRole } => m.user !== null)
          .map((m) => ({ user_id: m.user.id, role: m.role })),
      ]
      return createProject(name, description || undefined, (workflowId || defaultWorkflowId) ?? undefined, members)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projects'] })
      resetForm()
      setShowCreate(false)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'project.list.createFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!manager) {
      setError(t('project.list.managerRequired'))
      return
    }
    createMutation.mutate()
  }

  function addMemberRow() {
    setExtraMembers((rows) => [...rows, { user: null, role: 'member' }])
  }

  function updateMemberRow(index: number, patch: Partial<ExtraMemberRow>) {
    setExtraMembers((rows) => rows.map((row, i) => (i === index ? { ...row, ...patch } : row)))
  }

  function removeMemberRow(index: number) {
    setExtraMembers((rows) => rows.filter((_, i) => i !== index))
  }

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">{t('project.list.title')}</h1>
        {isAdmin && (
          <button
            onClick={() => setShowCreate(true)}
            className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
          >
            {t('common.new')}
          </button>
        )}
      </div>

      <div className="mb-4 flex gap-1 text-sm">
        <button
          onClick={() => {
            setArchivedTab(false)
            setPage(1)
          }}
          className={`rounded px-3 py-1.5 ${
            !archivedTab ? 'bg-blue-600 text-white' : 'bg-white text-gray-600 hover:bg-gray-50'
          } border border-gray-300`}
        >
          {t('project.list.tabActive')}
        </button>
        <button
          onClick={() => {
            setArchivedTab(true)
            setPage(1)
          }}
          className={`rounded px-3 py-1.5 ${
            archivedTab ? 'bg-blue-600 text-white' : 'bg-white text-gray-600 hover:bg-gray-50'
          } border border-gray-300`}
        >
          {t('project.list.tabArchived')}
        </button>
      </div>

      {isAdmin && (
        <div className="mb-4 flex gap-1 text-sm">
          <button
            onClick={() => {
              setScope('mine')
              setPage(1)
            }}
            className={`rounded px-3 py-1.5 ${
              scope === 'mine' ? 'bg-blue-600 text-white' : 'bg-white text-gray-600 hover:bg-gray-50'
            } border border-gray-300`}
          >
            {t('project.list.scopeMine')}
          </button>
          <button
            onClick={() => {
              setScope('all')
              setPage(1)
            }}
            className={`rounded px-3 py-1.5 ${
              scope === 'all' ? 'bg-blue-600 text-white' : 'bg-white text-gray-600 hover:bg-gray-50'
            } border border-gray-300`}
          >
            {t('project.list.scopeAll')}
          </button>
        </div>
      )}

      <Modal open={showCreate} onClose={() => setShowCreate(false)} title={t('project.list.createTitle')}>
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && <div className="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
          <input
            type="text"
            required
            placeholder={t('project.list.namePlaceholder')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full rounded border border-gray-300 px-3 py-2"
          />
          <input
            type="text"
            placeholder={t('project.list.descriptionPlaceholder')}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            className="w-full rounded border border-gray-300 px-3 py-2"
          />

          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">{t('project.list.workflowLabel')}</label>
            <select
              value={workflowId}
              onChange={(e) => setWorkflowId(e.target.value ? Number(e.target.value) : '')}
              className="w-full rounded border border-gray-300 px-3 py-2"
            >
              {workflows?.map((w) => (
                <option key={w.id} value={w.id}>
                  {w.name}
                  {w.is_default ? ` (${t('workflow.defaultBadge')})` : ''}
                </option>
              ))}
            </select>
            <p className="mt-1 text-xs text-gray-500">{t('project.list.workflowDescription')}</p>
          </div>

          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">{t('project.list.managerLabel')}</label>
            <UserPicker
              value={manager}
              onChange={setManager}
              placeholder={t('project.list.managerPlaceholder')}
              excludeIds={excludeIdsFor(manager?.id)}
            />
            <p className="mt-1 text-xs text-gray-500">{t('project.list.managerHint')}</p>
          </div>

          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">{t('project.list.membersLabel')}</label>
            <div className="space-y-2">
              {extraMembers.map((row, index) => (
                <div key={index} className="flex items-center gap-2">
                  <UserPicker
                    value={row.user}
                    onChange={(u) => updateMemberRow(index, { user: u })}
                    placeholder={t('project.list.selectUser')}
                    excludeIds={excludeIdsFor(row.user?.id)}
                    className="flex-1"
                  />
                  <select
                    value={row.role}
                    onChange={(e) => updateMemberRow(index, { role: e.target.value as ProjectRole })}
                    className="rounded border border-gray-300 px-2 py-1.5 text-sm"
                  >
                    {EXTRA_ROLE_OPTIONS.map((r) => (
                      <option key={r} value={r}>
                        {t(`projectMembers.role.${r}`)}
                      </option>
                    ))}
                  </select>
                  <button
                    type="button"
                    onClick={() => removeMemberRow(index)}
                    className="text-sm text-red-600 hover:underline"
                  >
                    {t('project.list.removeMemberRow')}
                  </button>
                </div>
              ))}
            </div>
            <button
              type="button"
              onClick={addMemberRow}
              className="mt-2 text-sm text-blue-600 hover:underline"
            >
              {t('project.list.addMemberRow')}
            </button>
            <p className="mt-1 text-xs text-gray-500">{t('project.list.membersHint')}</p>
          </div>

          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowCreate(false)}
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

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <ul className="divide-y rounded border bg-white">
          {projects?.map((project) => (
            <li key={project.id} className="flex items-center justify-between px-4 py-3 hover:bg-gray-50">
              <Link to={`/projects/${project.id}/tickets`} className="block flex-1">
                <div className="font-medium text-gray-800">
                  {project.name}
                  {project.archived_at && (
                    <span className="ml-2 rounded bg-gray-200 px-2 py-0.5 text-xs font-normal text-gray-600">
                      {t('project.list.archivedBadge')}
                    </span>
                  )}
                </div>
                {project.description && (
                  <div className="text-sm text-gray-500">{project.description}</div>
                )}
              </Link>
              {isAdmin && (
                <button
                  onClick={() =>
                    project.archived_at ? unarchiveMutation.mutate(project.id) : handleArchive(project.id)
                  }
                  className={`ml-2 shrink-0 text-sm hover:underline ${
                    project.archived_at ? 'text-green-600' : 'text-orange-600'
                  }`}
                >
                  {project.archived_at ? t('project.list.unarchive') : t('project.list.archive')}
                </button>
              )}
            </li>
          ))}
          {projects?.length === 0 && (
            <li className="px-4 py-6 text-center text-gray-400">
              {archivedTab ? t('project.list.emptyArchived') : t('project.list.empty')}
            </li>
          )}
        </ul>
      )}

      {data && (
        <Pagination
          page={page}
          perPage={perPage}
          total={data.total}
          onPageChange={setPage}
          onPerPageChange={(n) => {
            setPerPage(n)
            setPage(1)
          }}
        />
      )}
    </div>
  )
}
