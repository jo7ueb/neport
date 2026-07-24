import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { listAllProjects } from '../api/projects'
import { listAllProjectTickets } from '../api/tickets'
import { useAuth } from '../context/AuthContext'
import { WorkSessionWidget } from '../components/WorkSessionWidget'
import type { Ticket, WorkflowStatus } from '../types/models'

function isOverdue(dueDate: string | null, status: WorkflowStatus): boolean {
  if (!dueDate) return false
  if (status.category === 'done' || status.category === 'confirmed') return false
  return dueDate < new Date().toISOString().slice(0, 10)
}

interface TicketWithProject extends Ticket {
  projectName: string
}

export function DashboardPage() {
  const { t } = useTranslation()
  const { user } = useAuth()

  const { data: myTickets, isLoading } = useQuery({
    queryKey: ['dashboard', 'in-progress-tickets', user?.id],
    queryFn: async (): Promise<TicketWithProject[]> => {
      const projects = await listAllProjects()
      const perProject = await Promise.all(
        projects.map(async (project) => {
          const tickets = await listAllProjectTickets(project.id, {
            assignee_id: user!.id,
            category: 'in_progress',
          })
          return tickets.map((ticket) => ({ ...ticket, projectName: project.name }))
        })
      )
      return perProject.flat()
    },
    enabled: !!user,
  })

  return (
    <div>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('dashboard.title')}</h1>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="rounded border bg-white p-4">
          <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('dashboard.workTime')}</h2>
          <WorkSessionWidget />
        </div>

        <div>
          <div className="mb-4 rounded border bg-white p-4">
            <div className="mb-2 flex items-center justify-between">
              <h2 className="text-lg font-semibold text-gray-800">{t('dashboard.inProgressTickets')}</h2>
              <Link to="/projects" className="text-sm text-blue-600 hover:underline">
                {t('dashboard.projectList')}
              </Link>
            </div>
            {isLoading ? (
              <p className="text-sm text-gray-500">{t('common.loading')}</p>
            ) : (
              <ul className="divide-y">
                {myTickets?.map((t2) => (
                  <li key={t2.id} className="py-2">
                    <Link to={`/tickets/${t2.id}`} className="text-blue-600 hover:underline">
                      #{t2.id} {t2.title}
                    </Link>
                    <div className="text-xs text-gray-500">
                      {t2.projectName}
                      {t2.due_date && (
                        <span className={isOverdue(t2.due_date, t2.status) ? 'ml-2 font-semibold text-red-700' : 'ml-2'}>
                          {t('dashboard.dueLabel', { date: t2.due_date })}
                        </span>
                      )}
                    </div>
                  </li>
                ))}
                {myTickets?.length === 0 && (
                  <li className="py-4 text-center text-sm text-gray-400">
                    {t('dashboard.noInProgressTickets')}
                  </li>
                )}
              </ul>
            )}
          </div>

          <div className="rounded border bg-white p-4 text-center">
            <Link
              to="/daily-report"
              className="inline-block rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
            >
              {t('dashboard.writeDailyReport')}
            </Link>
          </div>
        </div>
      </div>
    </div>
  )
}
