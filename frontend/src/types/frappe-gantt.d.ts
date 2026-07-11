declare module 'frappe-gantt' {
  export interface GanttTask {
    id: string
    name: string
    start: string
    end: string
    progress?: number
    dependencies?: string
    custom_class?: string
    // Populated internally by the library once the task list is loaded -
    // not part of the input shape, but present by the time on_click/
    // on_date_change/custom_popup_html callbacks see the task.
    _start?: Date
    _end?: Date
  }

  export interface GanttOptions {
    view_mode?: 'Day' | 'Week' | 'Month'
    date_format?: string
    on_click?: (task: GanttTask) => void
    on_date_change?: (task: GanttTask, start: Date, end: Date) => void
    on_progress_change?: (task: GanttTask, progress: number) => void
    on_view_change?: (mode: string) => void
    custom_popup_html?: (task: GanttTask) => string
  }

  export default class Gantt {
    constructor(wrapper: string | HTMLElement, tasks: GanttTask[], options?: GanttOptions)
    refresh(tasks: GanttTask[]): void
  }
}
