import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { getProject, getProjectWorkflow } from '../api/projects'
import { listMilestones } from '../api/milestones'
import { listProjectTickets, type TicketSort } from '../api/tickets'
import { listAllUsers } from '../api/users'
import { ProjectNav } from '../components/ProjectNav'
import { Pagination } from '../components/Pagination'
import { TicketCreateModal } from '../components/TicketCreateModal'
import { TicketProgress } from '../components/TicketProgress'
import { statusLabel } from '../i18n/statusLabel'
import type { WorkflowStatus } from '../types/models'

function isOverdue(dueDate: string | null, status: WorkflowStatus): boolean {
  if (!dueDate) return false
  if (status.category === 'done' || status.category === 'confirmed') return false
  const today = new Date().toISOString().slice(0, 10)
  return dueDate < today
}

export function TicketListPage() {
  const { t } = useTranslation()
  const { projectId } = useParams()
  const pid = Number(projectId)
  const queryClient = useQueryClient()

  const [statusFilter, setStatusFilter] = useState<number | ''>('')
  const [assigneeFilter, setAssigneeFilter] = useState<number | ''>('')
  const [overdueOnly, setOverdueOnly] = useState(false)
  const [sort, setSort] = useState<TicketSort>('created_at_desc')
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useState(25)

  const { data: project } = useQuery({
    queryKey: ['project', pid],
    queryFn: () => getProject(pid),
  })
  const { data: workflow } = useQuery({
    queryKey: ['project', pid, 'workflow'],
    queryFn: () => getProjectWorkflow(pid),
  })
  const { data, isLoading } = useQuery({
    queryKey: ['tickets', pid, statusFilter, assigneeFilter, overdueOnly, sort, page, perPage],
    queryFn: () =>
      listProjectTickets(pid, {
        status_id: statusFilter || undefined,
        assignee_id: assigneeFilter || undefined,
        overdue: overdueOnly || undefined,
        sort,
        page,
        per_page: perPage,
      }),
  })
  const tickets = data?.items

  function updateStatusFilter(value: number | '') {
    setStatusFilter(value)
    setPage(1)
  }

  function updateAssigneeFilter(value: number | '') {
    setAssigneeFilter(value)
    setPage(1)
  }

  function updateOverdueOnly(value: boolean) {
    setOverdueOnly(value)
    setPage(1)
  }

  function updateSort(value: TicketSort) {
    setSort(value)
    setPage(1)
  }

  // Assignee filter needs the full user list, not a paginated page.
  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })
  const { data: milestones } = useQuery({ queryKey: ['milestones', pid], queryFn: () => listMilestones(pid) })

  const [showCreate, setShowCreate] = useState(false)

  function assigneeName(id: number | null): string {
    if (id === null) return t('common.unassigned')
    return users?.find((u) => u.id === id)?.display_name ?? `#${id}`
  }

  function milestoneName(id: number | null): string {
    if (id === null) return '-'
    return milestones?.find((m) => m.id === id)?.name ?? `#${id}`
  }

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">
          {project ? t('ticket.list.titleWithProject', { project: project.name }) : t('ticket.list.title')}
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
            {t('ticket.list.backToProjects')}
          </Link>
        </div>
      </div>

      <ProjectNav project={project} />

      <TicketCreateModal
        open={showCreate}
        onClose={() => setShowCreate(false)}
        projectId={pid}
        onCreated={() => {
          queryClient.invalidateQueries({ queryKey: ['tickets', pid] })
          queryClient.invalidateQueries({ queryKey: ['project', pid, 'ticket-summary'] })
          queryClient.invalidateQueries({ queryKey: ['milestones', pid] })
        }}
      />

      <div className="mb-4 flex flex-wrap items-center gap-3 rounded border bg-white p-3 text-sm">
        <label className="flex items-center gap-1">
          <span className="text-gray-600">{t('ticket.list.status')}</span>
          <select
            value={statusFilter}
            onChange={(e) => updateStatusFilter(e.target.value ? Number(e.target.value) : '')}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="">{t('ticket.list.all')}</option>
            {workflow?.statuses.map((s) => (
              <option key={s.id} value={s.id}>
                {statusLabel(t, s)}
              </option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-1">
          <span className="text-gray-600">{t('ticket.list.assignee')}</span>
          <select
            value={assigneeFilter}
            onChange={(e) => updateAssigneeFilter(e.target.value ? Number(e.target.value) : '')}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="">{t('ticket.list.all')}</option>
            {users?.map((u) => (
              <option key={u.id} value={u.id}>
                {u.display_name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-1">
          <input
            type="checkbox"
            checked={overdueOnly}
            onChange={(e) => updateOverdueOnly(e.target.checked)}
          />
          <span className="text-gray-600">{t('ticket.list.overdueOnly')}</span>
        </label>

        <label className="flex items-center gap-1">
          <span className="text-gray-600">{t('ticket.list.sort')}</span>
          <select
            value={sort}
            onChange={(e) => updateSort(e.target.value as TicketSort)}
            className="rounded border border-gray-300 px-2 py-1"
          >
            <option value="created_at_desc">{t('ticket.list.sortCreatedAtDesc')}</option>
            <option value="created_at_asc">{t('ticket.list.sortCreatedAtAsc')}</option>
            <option value="due_date_asc">{t('ticket.list.sortDueDateAsc')}</option>
            <option value="updated_at_desc">{t('ticket.list.sortUpdatedAtDesc')}</option>
          </select>
        </label>
      </div>

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <div className="overflow-x-auto rounded border bg-white">
          <table className="w-full text-sm">
            <thead className="border-b bg-gray-50 text-left text-gray-500">
              <tr>
                <th className="px-3 py-2">{t('ticket.list.columnId')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnTitle')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnStatus')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnStoryPoints')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnProgress')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnAssignee')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnMilestone')}</th>
                <th className="px-3 py-2">{t('ticket.list.columnDueDate')}</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {tickets?.map((ticket) => (
                <tr
                  key={ticket.id}
                  className={isOverdue(ticket.due_date, ticket.status) ? 'bg-red-50' : ''}
                >
                  <td className="px-3 py-2 text-gray-500">#{ticket.id}</td>
                  <td className="px-3 py-2">
                    <Link to={`/tickets/${ticket.id}`} className="text-blue-600 hover:underline">
                      {ticket.title}
                    </Link>
                  </td>
                  <td className="px-3 py-2">{statusLabel(t, ticket.status)}</td>
                  <td className="px-3 py-2">
                    {ticket.child_ticket_count > 0
                      ? t('ticket.detail.storyPointsNotApplicable')
                      : ticket.story_points ?? '-'}
                  </td>
                  <td className="px-3 py-2">
                    <TicketProgress done={ticket.child_done_ticket_count} total={ticket.child_ticket_count} />
                  </td>
                  <td className="px-3 py-2">{assigneeName(ticket.assignee_id)}</td>
                  <td className="px-3 py-2">{milestoneName(ticket.milestone_id)}</td>
                  <td className={`px-3 py-2 ${isOverdue(ticket.due_date, ticket.status) ? 'font-semibold text-red-700' : ''}`}>
                    {ticket.due_date ?? '-'}
                  </td>
                </tr>
              ))}
              {tickets?.length === 0 && (
                <tr>
                  <td colSpan={8} className="px-3 py-6 text-center text-gray-400">
                    {t('ticket.list.empty')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
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
