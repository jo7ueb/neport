import { ApiError, request } from './client'
import type { ClosureState, DecisionRecord } from '../types/models'

export type DayStatus = 'in_progress' | 'valid' | 'invalid' | 'approved' | 'rejected'

export interface WorkSession {
  id: number
  user_id: number
  started_at: string
  ended_at: string | null
  created_at: string
  updated_at: string
}

export interface DaySummary {
  date: string
  sessions: WorkSession[]
  first_started_at: string
  last_ended_at: string | null
  status: DayStatus
  decision: DecisionRecord | null
}

export interface MonthlyResponse {
  days: DaySummary[]
  closure: ClosureState
}

export function startSession(): Promise<WorkSession> {
  return request<WorkSession>('/work-sessions/start', { method: 'POST' })
}

export function endSession(): Promise<WorkSession> {
  return request<WorkSession>('/work-sessions/end', { method: 'POST' })
}

// No open session is a normal, expected state (not an error) - the caller
// needs to be able to tell "not working" apart from an actual failure.
export async function getCurrent(): Promise<WorkSession | null> {
  try {
    return await request<WorkSession>('/work-sessions/current')
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return null
    }
    throw err
  }
}

export function getToday(): Promise<DaySummary> {
  return request<DaySummary>('/work-sessions/today')
}

// No sessions on the given date is a normal, expected state (not an error).
export async function getDay(date: string): Promise<DaySummary | null> {
  try {
    return await request<DaySummary>('/work-sessions/day', { query: { date } })
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return null
    }
    throw err
  }
}

export function getMonthly(year: number, month: number): Promise<MonthlyResponse> {
  return request<MonthlyResponse>('/work-sessions/monthly', { query: { year, month } })
}

// Both fields are optional and leave the existing value untouched when
// omitted (see backend/src/work_sessions/handlers.rs::UpdateSessionRequest).
export function updateSession(
  id: number,
  fields: { started_at?: string; ended_at?: string }
): Promise<WorkSession> {
  return request<WorkSession>(`/work-sessions/${id}`, { method: 'PATCH', body: fields })
}
