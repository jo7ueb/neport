import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { listMilestones, getBurndown } from '../api/milestones'
import { getProject } from '../api/projects'
import { ProjectNav } from '../components/ProjectNav'
import { BurndownChart } from '../components/BurndownChart'

export function MilestoneBurndownPage() {
  const { t } = useTranslation()
  const { projectId, milestoneId } = useParams()
  const pid = Number(projectId)
  const mid = Number(milestoneId)

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: milestones } = useQuery({ queryKey: ['milestones', pid], queryFn: () => listMilestones(pid) })
  const milestone = milestones?.find((m) => m.id === mid)

  const { data, isLoading } = useQuery({
    queryKey: ['milestones', pid, mid, 'burndown'],
    queryFn: () => getBurndown(pid, mid),
  })

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {milestone ? t('burndown.titleWithMilestone', { milestone: milestone.name }) : t('burndown.title')}
        </h1>
        <Link to={`/projects/${pid}/milestones`} className="text-sm text-blue-600 hover:underline">
          {t('burndown.backToMilestones')}
        </Link>
      </div>

      <ProjectNav projectId={pid} archived={!!project?.archived_at} />

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : !data || !data.start_date ? (
        <div className="rounded border bg-white p-6 text-center text-gray-500">
          {t('burndown.noStartDate')}
        </div>
      ) : data.points.length === 0 ? (
        <div className="rounded border bg-white p-6 text-center text-gray-400">{t('burndown.noTickets')}</div>
      ) : (
        <div className="rounded border bg-white p-4">
          <BurndownChart points={data.points} totalAtStart={data.total_at_start} />
        </div>
      )}
    </div>
  )
}
