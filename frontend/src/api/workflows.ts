import { request } from './client'
import type { StatusCategory, Workflow, WorkflowDetail, WorkflowStatus, WorkflowTransition } from '../types/models'

export function listWorkflows(): Promise<Workflow[]> {
  return request<Workflow[]>('/workflows')
}

export function createWorkflow(name: string): Promise<Workflow> {
  return request<Workflow>('/workflows', { method: 'POST', body: { name } })
}

export function getWorkflow(id: number): Promise<WorkflowDetail> {
  return request<WorkflowDetail>(`/workflows/${id}`)
}

export function updateWorkflow(id: number, name: string): Promise<Workflow> {
  return request<Workflow>(`/workflows/${id}`, { method: 'PATCH', body: { name } })
}

export function deleteWorkflow(id: number): Promise<void> {
  return request<void>(`/workflows/${id}`, { method: 'DELETE' })
}

export interface CreateStatusInput {
  key: string
  label: string
  category: StatusCategory
  position: number
  is_initial?: boolean
}

export function createStatus(workflowId: number, input: CreateStatusInput): Promise<WorkflowStatus> {
  return request<WorkflowStatus>(`/workflows/${workflowId}/statuses`, { method: 'POST', body: input })
}

export interface UpdateStatusInput {
  key?: string
  label?: string
  category?: StatusCategory
  position?: number
  is_initial?: boolean
}

export function updateStatus(workflowId: number, statusId: number, input: UpdateStatusInput): Promise<WorkflowStatus> {
  return request<WorkflowStatus>(`/workflows/${workflowId}/statuses/${statusId}`, { method: 'PATCH', body: input })
}

export function deleteStatus(workflowId: number, statusId: number): Promise<void> {
  return request<void>(`/workflows/${workflowId}/statuses/${statusId}`, { method: 'DELETE' })
}

export function createTransition(
  workflowId: number,
  fromStatusId: number,
  toStatusId: number
): Promise<WorkflowTransition> {
  return request<WorkflowTransition>(`/workflows/${workflowId}/transitions`, {
    method: 'POST',
    body: { from_status_id: fromStatusId, to_status_id: toStatusId },
  })
}

export function deleteTransition(workflowId: number, transitionId: number): Promise<void> {
  return request<void>(`/workflows/${workflowId}/transitions/${transitionId}`, { method: 'DELETE' })
}
