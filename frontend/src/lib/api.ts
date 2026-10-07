export interface ApiEnvelope<T> {
  data: T
  meta?: Record<string, unknown>
  error: { code: string; message: string } | null
}

export class ApiError extends Error {
  constructor(public readonly code: string, message: string, public readonly status: number) {
    super(message)
  }
}

const baseUrl = import.meta.env.VITE_API_URL ?? ''

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = sessionStorage.getItem('access_token')
  let response = await fetch(`${baseUrl}${path}`, {
    ...init,
    signal: init.signal ?? AbortSignal.timeout(15_000),
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...init.headers,
    },
  })
  if (response.status === 401 && path !== '/api/v1/auth/refresh') {
    const refreshToken = localStorage.getItem('refresh_token')
    if (refreshToken) {
      const refreshResponse = await fetch(`${baseUrl}/api/v1/auth/refresh`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ refresh_token: refreshToken }) })
      if (refreshResponse.ok) {
        const refreshed = await refreshResponse.json() as ApiEnvelope<{ access_token: string; refresh_token: string }>
        sessionStorage.setItem('access_token', refreshed.data.access_token)
        localStorage.setItem('refresh_token', refreshed.data.refresh_token)
        response = await fetch(`${baseUrl}${path}`, { ...init, headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${refreshed.data.access_token}`, ...init.headers } })
      }
    }
  }
  const envelope = (await response.json()) as ApiEnvelope<T>
  if (!response.ok || envelope.error) {
    throw new ApiError(envelope.error?.code ?? 'REQUEST_FAILED', envelope.error?.message ?? 'Request failed', response.status)
  }
  return envelope.data
}

export function hasSession() { return Boolean(sessionStorage.getItem('access_token') || localStorage.getItem('refresh_token')) }
