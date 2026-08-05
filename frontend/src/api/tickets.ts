import { request } from './client'
import type {
  Paginated,
  StatusCategory,
  Ticket,
  TicketActivity,
  TicketComment,
  TicketDependencies,
  TicketDependency,
  TicketNode,
} from '../types/models'

// Internal "fetch everything" ceiling for call sites that need the full
// list (kanban board, dashboard aggregation) rather than a paginated UI page.
const FETCH_ALL_PER_PAGE = 500

export type TicketSort = 'created_at_desc' | 'created_at_asc' | 'due_date_asc' | 'updated_at_desc'

export interface TicketListFilter {
  assignee_id?: number
  status_id?: number
  category?: StatusCategory
  overdue?: boolean
  sort?: TicketSort
  page?: number
  per_page?: number
  [key: string]: string | number | boolean | undefined
}

export function listProjectTickets(projectId: number, filter: TicketListFilter = {}): Promise<Paginated<Ticket>> {
  return request<Paginated<Ticket>>(`/projects/${projectId}/tickets`, { query: filter })
}

export function listAllProjectTickets(projectId: number, filter: TicketListFilter = {}): Promise<Ticket[]> {
  return listProjectTickets(projectId, { ...filter, per_page: FETCH_ALL_PER_PAGE }).then((r) => r.items)
}

export function getTicket(id: number): Promise<Ticket> {
  return request<Ticket>(`/tickets/${id}`)
}

export function searchTickets(q: string): Promise<Ticket[]> {
  return request<Ticket[]>('/tickets/search', { query: { q } })
}

export function listTicketChildren(id: number): Promise<Ticket[]> {
  return request<Ticket[]>(`/tickets/${id}/children`)
}

export function getTicketTree(projectId: number): Promise<TicketNode[]> {
  return request<TicketNode[]>(`/projects/${projectId}/tickets/tree`)
}

export interface CreateTicketInput {
  project_id: number
  parent_ticket_id?: number | null
  assignee_id?: number | null
  title: string
  description?: string | null
  story_points?: number | null
  start_date?: string | null
  end_date?: string | null
  due_date?: string | null
  milestone_id?: number | null
}

export function createTicket(input: CreateTicketInput): Promise<Ticket> {
  return request<Ticket>('/tickets', { method: 'POST', body: input })
}

export interface UpdateTicketInput {
  title?: string
  description?: string | null
  story_points?: number | null
  status_id?: number
  assignee_id?: number | null
  start_date?: string | null
  end_date?: string | null
  due_date?: string | null
  parent_ticket_id?: number | null
  milestone_id?: number | null
}

export function updateTicket(id: number, input: UpdateTicketInput): Promise<Ticket> {
  return request<Ticket>(`/tickets/${id}`, { method: 'PATCH', body: input })
}

export function deleteTicket(id: number): Promise<void> {
  return request<void>(`/tickets/${id}`, { method: 'DELETE' })
}

export function listDependencies(ticketId: number): Promise<TicketDependencies> {
  return request<TicketDependencies>(`/tickets/${ticketId}/dependencies`)
}

export function createDependency(
  ticketId: number,
  direction: 'predecessor' | 'successor',
  otherTicketId: number
): Promise<TicketDependency> {
  return request<TicketDependency>(`/tickets/${ticketId}/dependencies`, {
    method: 'POST',
    body: { direction, other_ticket_id: otherTicketId },
  })
}

export function deleteDependency(ticketId: number, dependencyId: number): Promise<void> {
  return request<void>(`/tickets/${ticketId}/dependencies/${dependencyId}`, { method: 'DELETE' })
}

export function listComments(
  ticketId: number,
  params: { page?: number; per_page?: number } = {}
): Promise<Paginated<TicketComment>> {
  return request<Paginated<TicketComment>>(`/tickets/${ticketId}/comments`, { query: params })
}

export function createComment(ticketId: number, body: string): Promise<TicketComment> {
  return request<TicketComment>(`/tickets/${ticketId}/comments`, {
    method: 'POST',
    body: { body },
  })
}

export function listTicketActivity(
  ticketId: number,
  params: { page?: number; per_page?: number } = {}
): Promise<Paginated<TicketActivity>> {
  return request<Paginated<TicketActivity>>(`/tickets/${ticketId}/activity`, { query: params })
}

export function getActualDuration(ticketId: number): Promise<{ in_progress_seconds: number }> {
  return request<{ in_progress_seconds: number }>(`/tickets/${ticketId}/actual-duration`)
}
