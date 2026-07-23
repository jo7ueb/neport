export type UserRole = 'admin' | 'general'
export type UserStatus = 'active' | 'suspended' | 'deleted'

export type Locale = 'ja' | 'en'

export interface User {
  id: number
  email: string
  display_name: string
  role: UserRole
  status: UserStatus
  locale: Locale
  avatar_updated_at: string | null
  is_approver: boolean
}

export type ApprovalDecision = 'approved' | 'rejected'

export interface DecisionRecord {
  decision: ApprovalDecision
  comment: string
  approver_id: number
  approver_name: string
  decided_at: string
}

export type LockReason = 'approved' | 'month_closed'

export interface ClosureState {
  closed: boolean
  closed_by: number | null
  closed_by_name: string | null
  closed_at: string | null
}

export interface Project {
  id: number
  name: string
  description: string | null
  workflow_id: number
  created_at: string
  archived_at: string | null
}

export type ProjectRole = 'manager' | 'member' | 'viewer'

export interface ProjectMember {
  id: number
  project_id: number
  user: User
  role: ProjectRole
  created_at: string
}

export type StatusCategory = 'todo' | 'in_progress' | 'done'

export interface WorkflowStatus {
  id: number
  workflow_id: number
  key: string
  label: string
  category: StatusCategory
  position: number
  is_initial: boolean
}

export interface WorkflowTransition {
  id: number
  workflow_id: number
  from_status_id: number
  to_status_id: number
}

export interface Workflow {
  id: number
  name: string
  is_default: boolean
  created_at: string
}

export interface WorkflowDetail {
  workflow: Workflow
  statuses: WorkflowStatus[]
  transitions: WorkflowTransition[]
}

export interface Ticket {
  id: number
  project_id: number
  parent_ticket_id: number | null
  reporter_id: number
  assignee_id: number | null
  title: string
  description: string | null
  status: WorkflowStatus
  start_date: string | null
  end_date: string | null
  due_date: string | null
  milestone_id: number | null
  created_at: string
  updated_at: string
  child_ticket_count: number
  child_done_ticket_count: number
}

export type NotificationType =
  | 'ticket_assigned'
  | 'ticket_overdue'
  | 'ticket_commented'
  | 'report_approved'
  | 'report_rejected'

export interface Notification {
  id: number
  notif_type: NotificationType
  actor_id: number | null
  actor_name: string | null
  ticket_id: number | null
  ticket_title: string | null
  report_date: string | null
  read_at: string | null
  created_at: string
}

export type AuditAction =
  | 'user_suspended'
  | 'user_resumed'
  | 'user_deleted'
  | 'user_role_changed'
  | 'user_approver_flag_changed'
  | 'user_password_reset'
  | 'approver_removed'
  | 'project_archived'
  | 'project_unarchived'
  | 'project_member_removed'
  | 'system_settings_updated'
  | 'ticket_deleted'
  | 'workflow_deleted'
  | 'workflow_status_deleted'
  | 'workflow_transition_deleted'
  | 'login'
  | 'login_failed'
  | 'logout'
  | 'password_changed'
  | 'backup_created'
  | 'backup_restored'
  | 'user_created'
  | 'milestone_created'
  | 'milestone_updated'
  | 'milestone_deleted'
  | 'project_created'
  | 'project_updated'
  | 'project_member_added'
  | 'project_member_role_changed'
  | 'workflow_created'
  | 'workflow_updated'
  | 'workflow_status_created'
  | 'workflow_status_updated'
  | 'workflow_transition_created'

export interface AuditLogEntry {
  id: number
  actor_id: number
  actor_name: string
  action: AuditAction
  target_type: string
  target_id: number | null
  target_name: string | null
  from_text: string | null
  to_text: string | null
  ip_address: string | null
  occurred_at: string
}

export interface TicketComment {
  id: number
  ticket_id: number
  author_id: number
  body: string
  created_at: string
  updated_at: string | null
}

export type ActivityType = 'created' | 'comment' | 'status_change' | 'assignee_change' | 'field_update'

export interface TicketActivity {
  id: number
  ticket_id: number
  user_id: number
  activity_type: ActivityType
  occurred_at: string
  from_status_id: number | null
  to_status_id: number | null
  from_assignee_id: number | null
  to_assignee_id: number | null
  field_name: string | null
  from_text: string | null
  to_text: string | null
}

export interface TicketDependency {
  id: number
  predecessor_ticket_id: number
  successor_ticket_id: number
  created_at: string
  created_by_id: number | null
}

export interface DependentTicket extends Ticket {
  dependency_id: number
  dependency_created_at: string
  dependency_created_by_id: number | null
  dependency_deleted_at: string | null
  dependency_deleted_by_id: number | null
}

export interface TicketDependencies {
  predecessors: DependentTicket[]
  successors: DependentTicket[]
}

export interface TicketNode extends Ticket {
  children: TicketNode[]
}

export interface SystemSettings {
  katex_enabled: boolean
  default_timezone: string
  maintenance_mode: boolean
}

export interface Paginated<T> {
  items: T[]
  total: number
  page: number
  per_page: number
}
