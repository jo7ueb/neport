import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject, getProjectWorkflow } from '../api/projects'
import { getTicketTree } from '../api/tickets'
import { getGanttData } from '../api/gantt'
import { listMilestones } from '../api/milestones'
import { useAssignableUsers } from '../lib/useAssignableUsers'
import { ProjectNav } from '../components/ProjectNav'
import { WbsGanttPanel } from '../components/WbsGanttPanel'
import type { TicketNode } from '../types/models'

// Keeps a node if it belongs to the milestone itself, or if any of its
// descendants do (so a matching ticket's ancestors stay visible as context
// instead of being orphaned out of the tree).
function filterByMilestone(nodes: TicketNode[], milestoneId: number): TicketNode[] {
  const result: TicketNode[] = []
  for (const node of nodes) {
    const children = filterByMilestone(node.children, milestoneId)
    if (node.milestone_id === milestoneId || children.length > 0) {
      result.push({ ...node, children })
    }
  }
  return result
}

export function WbsPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)
  const [milestoneFilter, setMilestoneFilter] = useState<number | ''>('')

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: tree, isLoading } = useQuery({
    queryKey: ['project', pid, 'tickets', 'tree'],
    queryFn: () => getTicketTree(pid),
  })
  const { data: gantt } = useQuery({ queryKey: ['gantt', pid], queryFn: () => getGanttData(pid) })
  const { data: milestones } = useQuery({ queryKey: ['milestones', pid], queryFn: () => listMilestones(pid) })
  const users = useAssignableUsers(pid)
  const { data: workflow } = useQuery({
    queryKey: ['project', pid, 'workflow'],
    queryFn: () => getProjectWorkflow(pid),
  })

  const visibleTree = milestoneFilter === '' ? tree : filterByMilestone(tree ?? [], milestoneFilter)

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

      {milestones && milestones.length > 0 && (
        <div className="mb-4 flex items-center gap-2 text-sm">
          <label className="text-gray-600">{t('wbs.milestoneFilterLabel')}</label>
          <select
            value={milestoneFilter}
            onChange={(e) => setMilestoneFilter(e.target.value ? Number(e.target.value) : '')}
            className="rounded border border-gray-300 px-2 py-1.5"
          >
            <option value="">{t('wbs.milestoneFilterAll')}</option>
            {milestones.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <WbsGanttPanel
          projectId={pid}
          nodes={visibleTree ?? []}
          dependencies={gantt?.dependencies ?? []}
          users={users ?? []}
          workflow={workflow}
        />
      )}
    </div>
  )
}
