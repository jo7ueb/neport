import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject, getProjectWorkflow } from '../api/projects'
import { getTicketTree } from '../api/tickets'
import { listAllUsers } from '../api/users'
import { ProjectNav } from '../components/ProjectNav'
import { WbsTable } from '../components/WbsTable'

export function WbsPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: tree, isLoading } = useQuery({
    queryKey: ['project', pid, 'tickets', 'tree'],
    queryFn: () => getTicketTree(pid),
  })
  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })
  const { data: workflow } = useQuery({
    queryKey: ['project', pid, 'workflow'],
    queryFn: () => getProjectWorkflow(pid),
  })

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('wbs.titleWithProject', { project: project.name }) : t('wbs.title')}
        </h1>
        <Link to="/projects" className="text-sm text-blue-600 hover:underline">
          {t('wbs.backToProjects')}
        </Link>
      </div>

      <ProjectNav projectId={pid} archived={!!project?.archived_at} />

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <WbsTable projectId={pid} nodes={tree ?? []} users={users ?? []} workflow={workflow} />
      )}
    </div>
  )
}
