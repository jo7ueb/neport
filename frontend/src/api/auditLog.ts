import { request } from './client'
import type { AuditLogEntry, Paginated } from '../types/models'

export function listAuditLog(params: {
  user_id?: number
  from?: string
  to?: string
  page?: number
  per_page?: number
} = {}): Promise<Paginated<AuditLogEntry>> {
  return request<Paginated<AuditLogEntry>>('/audit-log', { query: params })
}
