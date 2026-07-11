import { ApiError, request } from './client'
import type { LockReason, Paginated, Ticket } from '../types/models'

export interface DailyReport {
  id: number
  user_id: number
  report_date: string
  achievement: string | null
  leftover: string | null
  keep: string | null
  problem: string | null
  try: string | null
  submitted_at: string
  created_at: string
  updated_at: string
}

export interface DailyReportView extends DailyReport {
  locked: boolean
  lock_reason: LockReason | null
  touched_tickets: Ticket[]
}

export function getTodayTickets(): Promise<Ticket[]> {
  return request<Ticket[]>('/daily-reports/today/tickets')
}

// A day with no report yet is a normal, expected state (not an error).
export async function getReport(date: string): Promise<DailyReportView | null> {
  try {
    return await request<DailyReportView>(`/daily-reports/${date}`)
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return null
    }
    throw err
  }
}

export interface SubmitReportInput {
  report_date?: string
  achievement?: string
  leftover?: string
  keep?: string
  problem?: string
  try?: string
}

export function submitReport(input: SubmitReportInput): Promise<DailyReport> {
  return request<DailyReport>('/daily-reports', { method: 'POST', body: input })
}

export function listReports(params: { page?: number; per_page?: number } = {}): Promise<Paginated<DailyReport>> {
  return request<Paginated<DailyReport>>('/daily-reports', { query: params })
}
