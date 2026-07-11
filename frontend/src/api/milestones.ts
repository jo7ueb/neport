import { request } from './client'

export interface Milestone {
  id: number
  project_id: number
  name: string
  start_date: string | null
  due_date: string | null
  description: string | null
  created_at: string
}

export interface BurndownPoint {
  date: string
  remaining: number
  ideal: number | null
}

export interface BurndownData {
  start_date: string | null
  due_date: string | null
  total_at_start: number
  points: BurndownPoint[]
}

export function listMilestones(projectId: number): Promise<Milestone[]> {
  return request<Milestone[]>(`/projects/${projectId}/milestones`)
}

export function createMilestone(
  projectId: number,
  name: string,
  dueDate?: string | null,
  description?: string | null,
  startDate?: string | null
): Promise<Milestone> {
  return request<Milestone>(`/projects/${projectId}/milestones`, {
    method: 'POST',
    body: { name, start_date: startDate || null, due_date: dueDate || null, description: description || null },
  })
}

export function updateMilestone(
  projectId: number,
  milestoneId: number,
  fields: { name?: string; start_date?: string; due_date?: string; description?: string }
): Promise<Milestone> {
  return request<Milestone>(`/projects/${projectId}/milestones/${milestoneId}`, {
    method: 'PATCH',
    body: fields,
  })
}

export function deleteMilestone(projectId: number, milestoneId: number): Promise<void> {
  return request<void>(`/projects/${projectId}/milestones/${milestoneId}`, { method: 'DELETE' })
}

export function getBurndown(projectId: number, milestoneId: number): Promise<BurndownData> {
  return request<BurndownData>(`/projects/${projectId}/milestones/${milestoneId}/burndown`)
}
