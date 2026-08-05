import { Navigate, Routes, Route } from 'react-router-dom'
import { ProtectedRoute } from './components/ProtectedRoute'
import { Layout } from './components/Layout'
import { LoginPage } from './pages/LoginPage'
import { DashboardPage } from './pages/DashboardPage'
import { ProjectListPage } from './pages/ProjectListPage'
import { TicketListPage } from './pages/TicketListPage'
import { TicketViewPage } from './pages/TicketViewPage'
import { TicketEditPage } from './pages/TicketEditPage'
import { KanbanPage } from './pages/KanbanPage'
import { MilestonesPage } from './pages/MilestonesPage'
import { MilestoneBurndownPage } from './pages/MilestoneBurndownPage'
import { WbsPage } from './pages/WbsPage'
import { ProjectMembersPage } from './pages/ProjectMembersPage'
import { AdminHubPage } from './pages/AdminHubPage'
import { AdminUsersPage } from './pages/AdminUsersPage'
import { AdminSettingsPage } from './pages/AdminSettingsPage'
import { AdminWorkflowsPage } from './pages/AdminWorkflowsPage'
import { AdminWorkflowDetailPage } from './pages/AdminWorkflowDetailPage'
import { AdminApproversPage } from './pages/AdminApproversPage'
import { AdminAuditLogPage } from './pages/AdminAuditLogPage'
import { AdminBackupPage } from './pages/AdminBackupPage'
import { AccountPage } from './pages/AccountPage'
import { WorkSessionMonthlyPage } from './pages/WorkSessionMonthlyPage'
import { DailyReportPage } from './pages/DailyReportPage'
import { DailyReportViewPage } from './pages/DailyReportViewPage'
import { ApprovalsPage } from './pages/ApprovalsPage'
import { ApprovalMonthlyPage } from './pages/ApprovalMonthlyPage'
import { ApprovalDecisionPage } from './pages/ApprovalDecisionPage'
import { NotFoundPage } from './pages/NotFoundPage'

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<ProtectedRoute />}>
        <Route element={<Layout />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/projects" element={<ProjectListPage />} />
          <Route path="/projects/:projectId" element={<Navigate to="wbs" replace />} />
          <Route path="/projects/:projectId/tickets" element={<TicketListPage />} />
          <Route path="/tickets/:ticketId" element={<TicketViewPage />} />
          <Route path="/tickets/:ticketId/edit" element={<TicketEditPage />} />
          <Route path="/projects/:projectId/kanban" element={<KanbanPage />} />
          <Route path="/projects/:projectId/milestones" element={<MilestonesPage />} />
          <Route path="/projects/:projectId/milestones/:milestoneId/burndown" element={<MilestoneBurndownPage />} />
          <Route path="/projects/:projectId/wbs" element={<WbsPage />} />
          <Route path="/projects/:projectId/members" element={<ProjectMembersPage />} />
          <Route path="/admin" element={<AdminHubPage />} />
          <Route path="/admin/users" element={<AdminUsersPage />} />
          <Route path="/admin/settings" element={<AdminSettingsPage />} />
          <Route path="/admin/workflows" element={<AdminWorkflowsPage />} />
          <Route path="/admin/workflows/:workflowId" element={<AdminWorkflowDetailPage />} />
          <Route path="/admin/approvers" element={<AdminApproversPage />} />
          <Route path="/admin/audit-log" element={<AdminAuditLogPage />} />
          <Route path="/admin/backup" element={<AdminBackupPage />} />
          <Route path="/account" element={<AccountPage />} />
          <Route path="/work/monthly" element={<WorkSessionMonthlyPage />} />
          <Route path="/daily-report" element={<DailyReportPage />} />
          <Route path="/daily-report/:date" element={<DailyReportViewPage />} />
          <Route path="/daily-report/:date/edit" element={<DailyReportPage />} />
          <Route path="/approvals" element={<ApprovalsPage />} />
          <Route path="/approvals/:userId/monthly" element={<ApprovalMonthlyPage />} />
          <Route path="/approvals/:userId/:date" element={<ApprovalDecisionPage />} />
        </Route>
      </Route>
      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  )
}
