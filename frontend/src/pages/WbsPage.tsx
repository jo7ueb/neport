import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject, getProjectWorkflow } from '../api/projects'
import { getTicketTree } from '../api/tickets'
import { getGanttData } from '../api/gantt'
import { listMilestones } from '../api/milestones'
import { listAllUsers } from '../api/users'
import { useAssignableUsers } from '../lib/useAssignableUsers'
import { ProjectNav } from '../components/ProjectNav'
import { WbsGanttPanel } from '../components/WbsGanttPanel'
import type { TicketNode, WorkflowStatus } from '../types/models'

// Mirrors TicketListPage's own isOverdue - a ticket in a done/confirmed
// status is never "overdue" regardless of its due date.
function isOverdue(dueDate: string | null, status: WorkflowStatus): boolean {
  if (!dueDate) return false
  if (status.category === 'done' || status.category === 'confirmed') return false
  return dueDate < new Date().toISOString().slice(0, 10)
}

// Keeps a node if it matches every active filter itself, or if any of its
// descendants do (so a matching ticket's ancestors stay visible as context
// instead of being orphaned out of the tree).
function filterTree(nodes: TicketNode[], predicate: (node: TicketNode) => boolean): TicketNode[] {
  const result: TicketNode[] = []
  for (const node of nodes) {
    const children = filterTree(node.children, predicate)
    if (predicate(node) || children.length > 0) {
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
  const [assigneeFilter, setAssigneeFilter] = useState<number | ''>('')
  const [overdueOnly, setOverdueOnly] = useState(false)

  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: tree, isLoading } = useQuery({
    queryKey: ['project', pid, 'tickets', 'tree'],
    queryFn: () => getTicketTree(pid),
  })
  const { data: gantt } = useQuery({ queryKey: ['gantt', pid], queryFn: () => getGanttData(pid) })
  const { data: milestones } = useQuery({ queryKey: ['milestones', pid], queryFn: () => listMilestones(pid) })
  // Who a ticket can be reassigned to in the panel's inline editor (project
  // members only - see useAssignableUsers) is a different question from who
  // the filter dropdown lets you filter by (every user, matching
  // TicketListPage's own assignee filter), so this is a separate query.
  const { data: filterUsers } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })
  const users = useAssignableUsers(pid)
  const { data: workflow } = useQuery({
    queryKey: ['project', pid, 'workflow'],
    queryFn: () => getProjectWorkflow(pid),
  })

  const visibleTree = filterTree(tree ?? [], (node) => {
    if (milestoneFilter !== '' && node.milestone_id !== milestoneFilter) return false
    if (assigneeFilter !== '' && node.assignee_id !== assigneeFilter) return false
    if (overdueOnly && !isOverdue(node.due_date, node.status)) return false
    return true
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

      <ProjectNav project={project} />

      <div className="mb-4 flex flex-wrap items-center gap-3 rounded border bg-white p-3 text-sm">
        {milestones && milestones.length > 0 && (
          <label className="flex items-center gap-1">
            <span className="text-gray-600">{t('wbs.milestoneFilterLabel')}</span>
            <select
              value={milestoneFilter}
              onChange={(e) => setMilestoneFilter(e.target.value ? Number(e.target.value) : '')}
              className="rounded border border-gray-300 px-2 py-1"
            >
              <option value="">{t('wbs.milestoneFilterAll')}</option>
              {milestones.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>
        )}

        <label className="flex items-center gap-1">
          <span className="text-gray-600">{t('ticket.list.assignee')}</span>
          <select
            value={assigneeFilter}
            onChange={(e) => setAssigneeFilter(e.target.value ? Number(e.target.value) : '')}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="">{t('ticket.list.all')}</option>
            {filterUsers?.map((u) => (
              <option key={u.id} value={u.id}>
                {u.display_name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-1">
          <input type="checkbox" checked={overdueOnly} onChange={(e) => setOverdueOnly(e.target.checked)} />
          <span className="text-gray-600">{t('ticket.list.overdueOnly')}</span>
        </label>
      </div>

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
