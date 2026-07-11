import { request } from './client'
import type { ProjectMember, ProjectRole } from '../types/models'

export function listProjectMembers(projectId: number): Promise<ProjectMember[]> {
  return request<ProjectMember[]>(`/projects/${projectId}/members`)
}

export function addProjectMember(projectId: number, userId: number, role: ProjectRole): Promise<ProjectMember> {
  return request<ProjectMember>(`/projects/${projectId}/members`, {
    method: 'POST',
    body: { user_id: userId, role },
  })
}

export function updateProjectMemberRole(projectId: number, userId: number, role: ProjectRole): Promise<void> {
  return request<void>(`/projects/${projectId}/members/${userId}`, {
    method: 'PATCH',
    body: { role },
  })
}

export function removeProjectMember(projectId: number, userId: number): Promise<void> {
  return request<void>(`/projects/${projectId}/members/${userId}`, { method: 'DELETE' })
}
