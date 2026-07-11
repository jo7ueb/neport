import { request } from './client'
import type { ApprovalDecision, ClosureState, DecisionRecord, User } from '../types/models'
import type { DaySummary, DayStatus } from './workSessions'
import type { DailyReport } from './dailyReports'

export interface SubordinateSummary extends User {
  pending_count: number
}

export function listSubordinates(): Promise<SubordinateSummary[]> {
  return request<SubordinateSummary[]>('/approvals/subordinates')
}

export interface SubordinateMonthlyResponse {
  days: DaySummary[]
  closure: ClosureState
}

export function getSubordinateMonthly(userId: number, year: number, month: number): Promise<SubordinateMonthlyResponse> {
  return request<SubordinateMonthlyResponse>(`/approvals/subordinates/${userId}/monthly`, { query: { year, month } })
}

export interface DayReview {
  report: DailyReport | null
  history: DecisionRecord[]
  status: DayStatus | null
}

export function getSubordinateDay(userId: number, date: string): Promise<DayReview> {
  return request<DayReview>(`/approvals/subordinates/${userId}/day/${date}`)
}

export function decide(userId: number, date: string, decision: ApprovalDecision, comment: string): Promise<DecisionRecord> {
  return request<DecisionRecord>(`/approvals/subordinates/${userId}/day/${date}/decide`, {
    method: 'POST',
    body: { decision, comment },
  })
}

export function closeMonth(userId: number, year: number, month: number): Promise<ClosureState> {
  return request<ClosureState>(`/approvals/subordinates/${userId}/month/${year}/${month}/close`, { method: 'POST' })
}

export function reopenMonth(userId: number, year: number, month: number): Promise<ClosureState> {
  return request<ClosureState>(`/approvals/subordinates/${userId}/month/${year}/${month}/reopen`, { method: 'POST' })
}
