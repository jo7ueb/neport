export class ApiError extends Error {
  status: number
  // Only present for the backend's two "this should not happen" error
  // variants (see backend/src/error.rs) - lets a user report a failure by
  // this id alone, which support/devs can then grep straight out of the
  // backend log instead of guessing from a timestamp.
  errorId?: string

  constructor(status: number, message: string, errorId?: string) {
    super(message)
    this.status = status
    this.errorId = errorId
  }
}

let csrfToken: string | null = null

export function setCsrfToken(token: string | null) {
  csrfToken = token
}

interface RequestOptions {
  method?: string
  body?: unknown
  query?: { [key: string]: string | number | boolean | undefined | null }
}

function buildQuery(query?: RequestOptions['query']): string {
  if (!query) return ''
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined && value !== null && value !== '') {
      params.set(key, String(value))
    }
  }
  const qs = params.toString()
  return qs ? `?${qs}` : ''
}

export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, query } = options

  const headers: Record<string, string> = {}
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }
  if (csrfToken && method !== 'GET') {
    headers['X-CSRF-Token'] = csrfToken
  }

  const response = await fetch(`/api/v1${path}${buildQuery(query)}`, {
    method,
    headers,
    credentials: 'include',
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })

  return handleResponse<T>(response)
}

// Separate from request() because a multipart body must NOT have a
// Content-Type header set manually (the browser needs to add its own
// boundary= parameter), whereas request() always forces application/json.
export async function requestFormData<T>(path: string, method: string, formData: FormData): Promise<T> {
  const headers: Record<string, string> = {}
  if (csrfToken) {
    headers['X-CSRF-Token'] = csrfToken
  }

  const response = await fetch(`/api/v1${path}`, {
    method,
    headers,
    credentials: 'include',
    body: formData,
  })

  return handleResponse<T>(response)
}

// For endpoints that return a binary file (e.g. a downloadable backup
// bundle) rather than JSON. Error responses are still JSON, so those are
// parsed and thrown the same way as handleResponse - only a successful
// response is read as a Blob instead of json().
export async function requestBlob(path: string, method: string): Promise<Blob> {
  const headers: Record<string, string> = {}
  if (csrfToken && method !== 'GET') {
    headers['X-CSRF-Token'] = csrfToken
  }

  const response = await fetch(`/api/v1${path}`, {
    method,
    headers,
    credentials: 'include',
  })

  if (!response.ok) {
    const isJson = response.headers.get('content-type')?.includes('application/json')
    const data = isJson ? await response.json() : undefined
    const message = (data && typeof data === 'object' && 'error' in data)
      ? String((data as { error: unknown }).error)
      : `Request failed with status ${response.status}`
    const errorId = (data && typeof data === 'object' && 'error_id' in data)
      ? String((data as { error_id: unknown }).error_id)
      : undefined
    throw new ApiError(response.status, message, errorId)
  }

  return response.blob()
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (response.status === 204) {
    return undefined as T
  }

  const isJson = response.headers.get('content-type')?.includes('application/json')
  const data = isJson ? await response.json() : undefined

  if (!response.ok) {
    const message = (data && typeof data === 'object' && 'error' in data)
      ? String((data as { error: unknown }).error)
      : `Request failed with status ${response.status}`
    const errorId = (data && typeof data === 'object' && 'error_id' in data)
      ? String((data as { error_id: unknown }).error_id)
      : undefined
    throw new ApiError(response.status, message, errorId)
  }

  return data as T
}
