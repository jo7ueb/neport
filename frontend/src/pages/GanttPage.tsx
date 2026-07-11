import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject } from '../api/projects'
import { getGanttData } from '../api/gantt'
import { GanttChart } from '../components/GanttChart'
import { ProjectNav } from '../components/ProjectNav'

export function GanttPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data, isLoading } = useQuery({ queryKey: ['gantt', pid], queryFn: () => getGanttData(pid) })

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('gantt.titleWithProject', { project: project.name }) : t('gantt.title')}
        </h1>
        <Link to="/projects" className="text-sm text-blue-600 hover:underline">
          {t('gantt.backToProjects')}
        </Link>
      </div>

      <ProjectNav projectId={pid} archived={!!project?.archived_at} />

      <div className="rounded border bg-white p-4">
        {isLoading ? (
          <p className="text-gray-500">{t('common.loading')}</p>
        ) : (
          <GanttChart projectId={pid} tickets={data?.tickets ?? []} dependencies={data?.dependencies ?? []} />
        )}
      </div>
    </div>
  )
}
