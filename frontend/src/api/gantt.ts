import { request } from './client'
import type { Ticket, TicketDependency } from '../types/models'

export interface GanttData {
  tickets: Ticket[]
  dependencies: TicketDependency[]
}

export function getGanttData(projectId: number): Promise<GanttData> {
  return request<GanttData>(`/projects/${projectId}/gantt`)
}
