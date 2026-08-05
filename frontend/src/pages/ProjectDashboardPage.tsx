import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject, getProjectTicketSummary } from '../api/projects'
import { listAllProjectTickets } from '../api/tickets'
import { listMilestones, type Milestone } from '../api/milestones'
import { listAllUsers } from '../api/users'
import { ProjectNav } from '../components/ProjectNav'
import { statusLabel } from '../i18n/statusLabel'
import { formatEstimatedHours } from '../lib/storyPoints'
import type { Ticket, WorkflowStatus } from '../types/models'

function isOverdue(ticket: Ticket): boolean {
  return !!ticket.due_date && ticket.due_date < new Date().toISOString().slice(0, 10) &&
    ticket.status.category !== 'done' && ticket.status.category !== 'confirmed'
}

function ProgressBar({ value }: { value: number }) {
  return (
    <div className="h-2 overflow-hidden rounded bg-gray-200">
      <div className="h-full rounded bg-blue-600" style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  )
}

export function ProjectDashboardPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)
  const { data: project } = useQuery({ queryKey: ['project', pid], queryFn: () => getProject(pid) })
  const { data: summary } = useQuery({
    queryKey: ['project', pid, 'ticket-summary'],
    queryFn: () => getProjectTicketSummary(pid),
  })
  const { data: tickets } = useQuery({
    queryKey: ['project', pid, 'tickets', 'dashboard'],
    queryFn: () => listAllProjectTickets(pid),
  })
  const { data: milestones } = useQuery({ queryKey: ['milestones', pid], queryFn: () => listMilestones(pid) })
  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })

  const activeTickets = useMemo(() => (tickets ?? []).filter((ticket) => ticket.status.category === 'in_progress'), [tickets])
  const overdueTickets = useMemo(() => (tickets ?? []).filter(isOverdue), [tickets])
  const statusCounts = useMemo(() => {
    const counts = new Map<number, { status: WorkflowStatus; count: number }>()
    for (const ticket of tickets ?? []) {
      const current = counts.get(ticket.status.id)
      counts.set(ticket.status.id, { status: ticket.status, count: (current?.count ?? 0) + 1 })
    }
    return [...counts.values()].sort((a, b) => a.status.position - b.status.position)
  }, [tickets])
  const userById = new Map((users ?? []).map((user) => [user.id, user]))
  const estimatedHours = summary ? formatEstimatedHours(summary.remaining_estimated_hours) : null
  const totalTickets = tickets?.length ?? 0
  const doneTickets = (tickets ?? []).filter((ticket) => ticket.status.category === 'done' || ticket.status.category === 'confirmed').length
  const completion = totalTickets === 0 ? 0 : (doneTickets / totalTickets) * 100

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('projectDashboard.titleWithProject', { project: project.name }) : t('projectDashboard.title')}
        </h1>
        <Link to="/projects" className="text-sm text-blue-600 hover:underline">{t('projectDashboard.backToProjects')}</Link>
      </div>
      <ProjectNav project={project} />

      <div className="mb-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <div className="rounded border bg-white p-4"><div className="text-sm text-gray-500">{t('projectDashboard.totalTickets')}</div><div className="mt-1 text-2xl font-semibold">{totalTickets}</div></div>
        <div className="rounded border bg-white p-4"><div className="text-sm text-gray-500">{t('projectDashboard.completion')}</div><div className="mt-1 text-2xl font-semibold">{Math.round(completion)}%</div><ProgressBar value={completion} /></div>
        <div className="rounded border bg-white p-4"><div className="text-sm text-gray-500">{t('projectDashboard.remainingStoryPoints')}</div><div className="mt-1 text-2xl font-semibold">{summary?.remaining_story_points ?? '-'}</div></div>
        <div className="rounded border bg-white p-4"><div className="text-sm text-gray-500">{t('projectDashboard.remainingEstimatedHours')}</div><div className="mt-1 text-2xl font-semibold">{estimatedHours ? t('workSessionMonthly.hoursMinutes', estimatedHours) : '-'}</div></div>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <section className="rounded border bg-white p-4">
          <div className="mb-3 flex items-center justify-between"><h2 className="text-lg font-semibold">{t('projectDashboard.statusOverview')}</h2><Link to={`/projects/${pid}/wbs`} className="text-sm text-blue-600 hover:underline">{t('projectDashboard.openWbs')}</Link></div>
          <div className="space-y-3">
            {statusCounts.map(({ status, count }) => <div key={status.id}><div className="mb-1 flex justify-between text-sm"><span>{statusLabel(t, status)}</span><span className="text-gray-500">{count}</span></div><ProgressBar value={totalTickets ? (count / totalTickets) * 100 : 0} /></div>)}
            {statusCounts.length === 0 && <p className="text-sm text-gray-400">{t('projectDashboard.noTickets')}</p>}
          </div>
        </section>

        <section className="rounded border bg-white p-4">
          <h2 className="mb-3 text-lg font-semibold">{t('projectDashboard.activeWork')}</h2>
          {activeTickets.length === 0 ? <p className="text-sm text-gray-400">{t('projectDashboard.noActiveWork')}</p> : <div className="space-y-2">{activeTickets.slice(0, 8).map((ticket) => <div key={ticket.id} className="flex items-center justify-between gap-3 border-b pb-2 text-sm last:border-0"><Link to={`/tickets/${ticket.id}`} className="min-w-0 truncate text-blue-600 hover:underline">#{ticket.id} {ticket.title}</Link><span className="shrink-0 text-gray-500">{userById.get(ticket.assignee_id ?? 0)?.display_name ?? t('projectDashboard.unassigned')}</span></div>)}</div>}
        </section>

        <section className="rounded border bg-white p-4">
          <div className="mb-3 flex items-center justify-between"><h2 className="text-lg font-semibold">{t('projectDashboard.overdue')}</h2><Link to={`/projects/${pid}/wbs`} className="text-sm text-blue-600 hover:underline">{t('projectDashboard.openWbs')}</Link></div>
          {overdueTickets.length === 0 ? <p className="text-sm text-gray-400">{t('projectDashboard.noOverdue')}</p> : <div className="space-y-2">{overdueTickets.slice(0, 8).map((ticket) => <div key={ticket.id} className="flex items-center justify-between gap-3 border-b pb-2 text-sm last:border-0"><Link to={`/tickets/${ticket.id}`} className="min-w-0 truncate text-blue-600 hover:underline">#{ticket.id} {ticket.title}</Link><span className="shrink-0 font-medium text-red-700">{ticket.due_date}</span></div>)}</div>}
        </section>

        <section className="rounded border bg-white p-4">
          <h2 className="mb-3 text-lg font-semibold">{t('projectDashboard.milestones')}</h2>
          {milestones?.length ? <div className="space-y-3">{milestones.slice(0, 6).map((milestone: Milestone) => { const progress = milestone.total_story_points ? (milestone.done_story_points / milestone.total_story_points) * 100 : 0; return <div key={milestone.id}><div className="mb-1 flex justify-between text-sm"><Link to={`/projects/${pid}/milestones/${milestone.id}/burndown`} className="text-blue-600 hover:underline">{milestone.name}</Link><span className="text-gray-500">{Math.round(progress)}%</span></div><ProgressBar value={progress} /></div> })}</div> : <p className="text-sm text-gray-400">{t('projectDashboard.noMilestones')}</p>}
        </section>
      </div>
    </div>
  )
}
