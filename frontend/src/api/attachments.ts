import { requestFormData } from './client'

export interface UploadedAttachment {
  url: string
}

export function uploadAttachment(file: File | Blob): Promise<UploadedAttachment> {
  const formData = new FormData()
  formData.append('file', file)
  return requestFormData<UploadedAttachment>('/attachments', 'POST', formData)
}
