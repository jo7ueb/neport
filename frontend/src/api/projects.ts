import { request } from './client'
import type { Paginated, Project, ProjectRole, WorkflowDetail } from '../types/models'

export interface InitialMemberInput {
  user_id: number
  role: ProjectRole
}

// Internal "fetch everything" ceiling for call sites that need the full
// list (e.g. dashboard aggregation) rather than a paginated UI page.
const FETCH_ALL_PER_PAGE = 500

export function listProjects(
  params: { page?: number; per_page?: number; scope?: 'mine' | 'all'; archived?: boolean } = {}
): Promise<Paginated<Project>> {
  return request<Paginated<Project>>('/projects', { query: params })
}

// Internal aggregation call sites (e.g. the dashboard) need every project
// visible to the caller regardless of membership, not just "my" projects -
// `scope: 'all'` is a no-op for non-admins (the backend ignores it unless
// the caller is a site admin) and restores full visibility for admins.
export function listAllProjects(): Promise<Project[]> {
  return listProjects({ per_page: FETCH_ALL_PER_PAGE, scope: 'all' }).then((r) => r.items)
}

export function getProject(id: number): Promise<Project> {
  return request<Project>(`/projects/${id}`)
}

export function createProject(
  name: string,
  description?: string,
  workflowId?: number,
  members?: InitialMemberInput[],
): Promise<Project> {
  return request<Project>('/projects', {
    method: 'POST',
    body: { name, description: description || null, workflow_id: workflowId, members },
  })
}

export function getProjectWorkflow(id: number): Promise<WorkflowDetail> {
  return request<WorkflowDetail>(`/projects/${id}/workflow`)
}

export function updateProject(
  id: number,
  fields: { name?: string; description?: string }
): Promise<Project> {
  return request<Project>(`/projects/${id}`, {
    method: 'PATCH',
    body: fields,
  })
}

export function archiveProject(id: number): Promise<void> {
  return request<void>(`/projects/${id}/archive`, { method: 'POST' })
}

export function unarchiveProject(id: number): Promise<void> {
  return request<void>(`/projects/${id}/unarchive`, { method: 'POST' })
}
