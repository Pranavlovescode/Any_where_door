export type AppTheme = 'dark' | 'light'

export type ApiFile = {
  file_id: string
  file_name: string
  file_path: string
  file_size: number
  file_hash: string
  mime_type: string
  uploaded_at: number
}

export type ExplorerNode = {
  name: string
  path: string
  files: ApiFile[]
  folders: Map<string, ExplorerNode>
}

export type LoginResponse = {
  jwt: string
  user_id: string
  expires_in: number
}

export type CachedAuth = {
  jwt: string
  userId: string
  serverUrl: string
}
