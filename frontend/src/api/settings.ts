import { request } from './client'
import type { SystemSettings } from '../types/models'

export function getSettings(): Promise<SystemSettings> {
  return request<SystemSettings>('/settings')
}

export function updateSettings(settings: SystemSettings): Promise<SystemSettings> {
  return request<SystemSettings>('/settings', {
    method: 'PATCH',
    body: settings,
  })
}
