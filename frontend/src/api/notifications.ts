import { request } from './client'
import type { Notification, Paginated } from '../types/models'

export function listNotifications(params: { unread_only?: boolean; page?: number; per_page?: number } = {}): Promise<Paginated<Notification>> {
  return request<Paginated<Notification>>('/notifications', { query: params })
}

export function getUnreadCount(): Promise<{ count: number }> {
  return request<{ count: number }>('/notifications/unread-count')
}

export function markNotificationRead(id: number): Promise<void> {
  return request<void>(`/notifications/${id}/read`, { method: 'POST' })
}

export function markAllNotificationsRead(): Promise<void> {
  return request<void>('/notifications/read-all', { method: 'POST' })
}
