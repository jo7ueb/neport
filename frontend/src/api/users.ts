import { request } from './client'
import type { Paginated, User, UserRole } from '../types/models'

// Internal "fetch everything" ceiling for call sites that need the full
// list (assignee dropdowns) rather than a paginated UI page.
const FETCH_ALL_PER_PAGE = 500

export function listUsers(params: { page?: number; per_page?: number } = {}): Promise<Paginated<User>> {
  return request<Paginated<User>>('/users', { query: params })
}

export function listAllUsers(): Promise<User[]> {
  return listUsers({ per_page: FETCH_ALL_PER_PAGE }).then((r) => r.items)
}

export function searchUsers(q: string): Promise<User[]> {
  return request<User[]>('/users/search', { query: { q } })
}

export function createUser(
  email: string,
  password: string,
  displayName: string,
  role: UserRole,
): Promise<User> {
  return request<User>('/users', {
    method: 'POST',
    body: { email, password, display_name: displayName, role },
  })
}

export function suspendUser(id: number): Promise<void> {
  return request<void>(`/users/${id}/suspend`, { method: 'POST' })
}

export function resumeUser(id: number): Promise<void> {
  return request<void>(`/users/${id}/resume`, { method: 'POST' })
}

export function resetUserPassword(id: number): Promise<{ temporary_password: string }> {
  return request<{ temporary_password: string }>(`/users/${id}/reset-password`, { method: 'POST' })
}

export function updateUserRole(id: number, role: UserRole): Promise<void> {
  return request<void>(`/users/${id}/role`, { method: 'PATCH', body: { role } })
}

export function updateIsApprover(id: number, isApprover: boolean): Promise<void> {
  return request<void>(`/users/${id}/is-approver`, { method: 'PATCH', body: { is_approver: isApprover } })
}

export function listApprovers(userId: number): Promise<User[]> {
  return request<User[]>(`/users/${userId}/approvers`)
}

export function addApprover(userId: number, approverId: number): Promise<void> {
  return request<void>(`/users/${userId}/approvers`, { method: 'POST', body: { approver_id: approverId } })
}

export function removeApprover(userId: number, approverId: number): Promise<void> {
  return request<void>(`/users/${userId}/approvers/${approverId}`, { method: 'DELETE' })
}
