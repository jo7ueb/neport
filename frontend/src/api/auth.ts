import { request, requestFormData } from './client'
import type { Locale, User } from '../types/models'

export interface LoginResponse {
  user: User
  csrf_token: string
}

export function fetchMe(): Promise<LoginResponse> {
  return request<LoginResponse>('/auth/me')
}

export function login(email: string, password: string): Promise<LoginResponse> {
  return request<LoginResponse>('/auth/login', {
    method: 'POST',
    body: { email, password },
  })
}

export function logout(): Promise<void> {
  return request<void>('/auth/logout', { method: 'POST' })
}

export function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  return request<void>('/auth/password', {
    method: 'PATCH',
    body: { current_password: currentPassword, new_password: newPassword },
  })
}

export function updateLocale(locale: Locale): Promise<void> {
  return request<void>('/auth/locale', {
    method: 'PATCH',
    body: { locale },
  })
}

export interface TimezoneResponse {
  timezone: string | null
  effective_timezone: string
}

export function getTimezone(): Promise<TimezoneResponse> {
  return request<TimezoneResponse>('/auth/timezone')
}

// null clears the override and reverts to the system default.
export function updateTimezone(timezone: string | null): Promise<TimezoneResponse> {
  return request<TimezoneResponse>('/auth/timezone', {
    method: 'PATCH',
    body: { timezone },
  })
}

export function updateProfile(displayName: string): Promise<User> {
  return request<User>('/auth/profile', {
    method: 'PATCH',
    body: { display_name: displayName },
  })
}

export function uploadAvatar(file: File): Promise<User> {
  const formData = new FormData()
  formData.append('file', file)
  return requestFormData<User>('/auth/avatar', 'POST', formData)
}

export function deleteAvatar(): Promise<User> {
  return request<User>('/auth/avatar', { method: 'DELETE' })
}

export function avatarUrl(userId: number, avatarUpdatedAt: string | null): string | null {
  return avatarUpdatedAt ? `/api/v1/users/${userId}/avatar?v=${encodeURIComponent(avatarUpdatedAt)}` : null
}
