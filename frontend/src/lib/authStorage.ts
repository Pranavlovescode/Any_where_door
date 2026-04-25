import { AUTH_STORAGE_KEY, DEFAULT_SERVER_URL } from '../constants/storage'
import type { CachedAuth } from '../types/models'

export const getCachedAuth = (): CachedAuth => {
  const fallback: CachedAuth = { jwt: '', userId: '', serverUrl: DEFAULT_SERVER_URL }
  const cached = localStorage.getItem(AUTH_STORAGE_KEY)
  if (!cached) {
    return fallback
  }

  try {
    const parsed = JSON.parse(cached) as Partial<CachedAuth>
    return {
      jwt: parsed.jwt ?? '',
      userId: parsed.userId ?? '',
      serverUrl: parsed.serverUrl || DEFAULT_SERVER_URL,
    }
  } catch {
    localStorage.removeItem(AUTH_STORAGE_KEY)
    return fallback
  }
}

export const storeAuth = (auth: CachedAuth): void => {
  localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(auth))
}

export const clearCachedAuth = (): void => {
  localStorage.removeItem(AUTH_STORAGE_KEY)
}
