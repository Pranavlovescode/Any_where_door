import type { ApiFile, LoginResponse } from '../types/models'

const responseError = async (response: Response, fallback: string): Promise<Error> => {
  try {
    const payload = (await response.json()) as { detail?: string }
    return new Error(payload.detail ?? fallback)
  } catch {
    return new Error(fallback)
  }
}

export const loginUser = async (
  serverUrl: string,
  username: string,
  password: string,
): Promise<LoginResponse> => {
  const response = await fetch(`${serverUrl}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })

  if (!response.ok) {
    throw await responseError(response, 'Login failed')
  }

  return (await response.json()) as LoginResponse
}

export const createUser = async (
  serverUrl: string,
  username: string,
  password: string,
): Promise<void> => {
  const params = new URLSearchParams({ username, password })
  const response = await fetch(`${serverUrl}/auth/create-user?${params.toString()}`, {
    method: 'POST',
  })

  if (!response.ok) {
    throw await responseError(response, 'User creation failed')
  }
}

export const listFiles = async (serverUrl: string, jwt: string): Promise<ApiFile[]> => {
  const response = await fetch(
    `${serverUrl}/api/files/list?jwt=${encodeURIComponent(jwt)}&limit=1000&skip=0`,
  )

  if (!response.ok) {
    throw await responseError(response, 'Failed to fetch files')
  }

  const payload = (await response.json()) as { files?: ApiFile[] }
  return payload.files ?? []
}

export const uploadFile = async (
  serverUrl: string,
  jwt: string,
  payload: unknown,
): Promise<void> => {
  const response = await fetch(`${serverUrl}/api/files/upload?jwt=${encodeURIComponent(jwt)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })

  if (!response.ok) {
    throw await responseError(response, 'Upload failed')
  }
}

export const downloadFileContent = async (
  serverUrl: string,
  jwt: string,
  fileId: string,
): Promise<{ file_name: string; file_content: string; mime_type: string }> => {
  const response = await fetch(
    `${serverUrl}/api/files/${fileId}/download?jwt=${encodeURIComponent(jwt)}`,
  )

  if (!response.ok) {
    throw await responseError(response, 'Download failed')
  }

  return (await response.json()) as {
    file_name: string
    file_content: string
    mime_type: string
  }
}

export const deleteFileRemote = async (
  serverUrl: string,
  jwt: string,
  fileId: string,
): Promise<void> => {
  const response = await fetch(`${serverUrl}/api/files/${fileId}?jwt=${encodeURIComponent(jwt)}`, {
    method: 'DELETE',
  })

  if (!response.ok) {
    throw await responseError(response, 'Delete failed')
  }
}
