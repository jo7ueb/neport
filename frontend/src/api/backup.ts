import { requestBlob, requestFormData } from './client'

export function createBackup(): Promise<Blob> {
  return requestBlob('/backup', 'POST')
}

export function restoreBackup(file: File): Promise<void> {
  const formData = new FormData()
  formData.append('file', file)
  return requestFormData<void>('/backup/restore', 'POST', formData)
}
