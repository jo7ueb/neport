import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject } from '../api/projects'
import { addProjectMember, listProjectMembers, removeProjectMember, updateProjectMemberRole } from '../api/projectMembers'
import { listAllUsers } from '../api/users'
import { ProjectNav } from '../components/ProjectNav'
import { UserAvatar } from '../components/UserAvatar'
import { getErrorMessage } from '../i18n/apiErrors'
import { useProjectRole } from '../lib/useProjectRole'
import type { ProjectRole } from '../types/models'

const ROLE_OPTIONS: ProjectRole[] = ['manager', 'member', 'viewer']

export function ProjectMembersPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)
  const queryClient = useQueryClient()

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: members, isLoading } = useQuery({
    queryKey: ['project', pid, 'members'],
    queryFn: () => listProjectMembers(pid),
  })
  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['project', pid, 'members'] })

  const [error, setError] = useState<string | null>(null)
  const [newUserId, setNewUserId] = useState<number | ''>('')
  const [newRole, setNewRole] = useState<ProjectRole>('member')

  const addMutation = useMutation({
    mutationFn: () => addProjectMember(pid, Number(newUserId), newRole),
    onSuccess: () => {
      invalidate()
      setNewUserId('')
      setNewRole('member')
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'projectMembers.addFailed')),
  })

  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: number; role: ProjectRole }) => updateProjectMemberRole(pid, userId, role),
    onSuccess: () => {
      invalidate()
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'projectMembers.updateFailed')),
  })

  const removeMutation = useMutation({
    mutationFn: (userId: number) => removeProjectMember(pid, userId),
    onSuccess: () => {
      invalidate()
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'projectMembers.removeFailed')),
  })

  function handleAddSubmit(e: FormEvent) {
    e.preventDefault()
    if (newUserId === '') return
    addMutation.mutate()
  }

  // On a legacy project with no members at all, Read access is open to
  // everyone but Manage still requires an explicit manager (or site admin)
  // - so with zero members, nobody but a site admin can see these controls
  // until someone bootstraps membership. See backend/src/access.rs.
  const { canManage: canManageRole } = useProjectRole(pid)
  const canManage = canManageRole && !project?.archived_at

  const availableUsers = users?.filter((u) => !members?.some((m) => m.user.id === u.id))

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('projectMembers.titleWithProject', { project: project.name }) : t('projectMembers.title')}
        </h1>
        <Link to="/projects" className="text-sm text-blue-600 hover:underline">
          {t('projectMembers.backToProjects')}
        </Link>
      </div>

      {project?.description && <p className="mb-4 text-sm text-gray-600">{project.description}</p>}

      <ProjectNav project={project} />

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <div className="rounded border bg-white p-4">
          {members?.length === 0 && <p className="mb-3 text-sm text-gray-400">{t('projectMembers.legacyOpenNotice')}</p>}
          <ul className="mb-4 space-y-2">
            {members?.map((member) => (
              <li key={member.id} className="flex items-center justify-between gap-2 rounded border px-3 py-2 text-sm">
                <div className="flex items-center gap-2">
                  <UserAvatar user={member.user} size={28} />
                  <div>
                    <div className="font-medium text-gray-800">{member.user.display_name}</div>
                    <div className="text-xs text-gray-500">{member.user.email}</div>
                  </div>
                </div>
                {canManage ? (
                  <div className="flex items-center gap-2">
                    <select
                      value={member.role}
                      onChange={(e) =>
                        updateRoleMutation.mutate({ userId: member.user.id, role: e.target.value as ProjectRole })
                      }
                      disabled={updateRoleMutation.isPending}
                      className="rounded border border-gray-300 px-2 py-1 text-sm"
                    >
                      {ROLE_OPTIONS.map((r) => (
                        <option key={r} value={r}>
                          {t(`projectMembers.role.${r}`)}
                        </option>
                      ))}
                    </select>
                    <button
                      onClick={() => removeMutation.mutate(member.user.id)}
                      disabled={removeMutation.isPending}
                      className="text-red-600 hover:underline disabled:opacity-50"
                    >
                      {t('projectMembers.remove')}
                    </button>
                  </div>
                ) : (
                  <span className="text-gray-600">{t(`projectMembers.role.${member.role}`)}</span>
                )}
              </li>
            ))}
          </ul>

          {canManage && (
            <form onSubmit={handleAddSubmit} className="flex flex-wrap items-end gap-2 text-sm">
              <label>
                <span className="mb-1 block text-gray-600">{t('projectMembers.addUser')}</span>
                <select
                  value={newUserId}
                  onChange={(e) => setNewUserId(e.target.value ? Number(e.target.value) : '')}
                  className="rounded border border-gray-300 px-2 py-1"
                >
                  <option value="">{t('projectMembers.selectUser')}</option>
                  {availableUsers?.map((u) => (
                    <option key={u.id} value={u.id}>
                      {u.display_name} ({u.email})
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span className="mb-1 block text-gray-600">{t('projectMembers.role.label')}</span>
                <select
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value as ProjectRole)}
                  className="rounded border border-gray-300 px-2 py-1"
                >
                  {ROLE_OPTIONS.map((r) => (
                    <option key={r} value={r}>
                      {t(`projectMembers.role.${r}`)}
                    </option>
                  ))}
                </select>
              </label>
              <button
                type="submit"
                disabled={addMutation.isPending || newUserId === ''}
                className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {t('projectMembers.add')}
              </button>
            </form>
          )}
        </div>
      )}
    </div>
  )
}
