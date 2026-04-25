import { useCallback, useMemo, useState } from 'react'
import { DEFAULT_SERVER_URL } from '../constants/storage'
import {
  deleteFileRemote,
  downloadFileContent,
  listFiles,
  uploadFile,
} from '../lib/api'
import { clearCachedAuth, getCachedAuth, storeAuth } from '../lib/authStorage'
import { sha256Hex, toBase64 } from '../lib/fileCodec'
import { buildTree, flattenFolders } from '../lib/fileTree'
import type { ApiFile } from '../types/models'

export const useDashboard = () => {
  const initialAuth = getCachedAuth()

  const [serverUrl, setServerUrl] = useState(initialAuth.serverUrl || DEFAULT_SERVER_URL)
  const [jwt, setJwt] = useState(initialAuth.jwt)
  const [userId, setUserId] = useState(initialAuth.userId)
  const [files, setFiles] = useState<ApiFile[]>([])
  const [selectedFolder, setSelectedFolder] = useState('')
  const [selectedFileId, setSelectedFileId] = useState<string | null>(null)
  const [info, setInfo] = useState('')
  const [error, setError] = useState('')
  const [isFetchingFiles, setIsFetchingFiles] = useState(false)
  const [isUploading, setIsUploading] = useState(false)

  const tree = useMemo(() => buildTree(files), [files])
  const folders = useMemo(() => flattenFolders(tree), [tree])
  const selectedNode = useMemo(
    () => folders.find((folder) => folder.path === selectedFolder) ?? tree,
    [folders, selectedFolder, tree],
  )

  const selectedFile = useMemo(
    () => selectedNode.files.find((file) => file.file_id === selectedFileId) ?? null,
    [selectedNode.files, selectedFileId],
  )

  const clearMessages = () => {
    setInfo('')
    setError('')
  }

  const isAuthenticated = jwt.length > 0

  const fetchFiles = useCallback(async (token = jwt) => {
    if (!token) return
    clearMessages()
    setIsFetchingFiles(true)

    try {
      const nextFiles = await listFiles(serverUrl, token)
      setFiles(nextFiles)
      setSelectedFolder('')
      setSelectedFileId(nextFiles[0]?.file_id ?? null)
      setInfo(`Synced ${nextFiles.length} files from server.`)
      storeAuth({ jwt: token, userId, serverUrl })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not load files')
    } finally {
      setIsFetchingFiles(false)
    }
  }, [jwt, serverUrl, userId])

  const uploadFiles = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const selection = event.target.files
    if (!selection?.length || !jwt) return

    clearMessages()
    setIsUploading(true)
    const selectedFiles = Array.from(selection)

    try {
      for (const localFile of selectedFiles) {
        const base64 = await toBase64(localFile)
        const fileHash = await sha256Hex(localFile)
        const nowEpoch = Math.floor(Date.now() / 1000)
        const pathPrefix = selectedFolder ? `${selectedFolder}/` : ''

        const payload = {
          metadata: {
            file_path: `${pathPrefix}${localFile.name}`,
            file_name: localFile.name,
            file_size: localFile.size,
            modified_at: Math.floor(localFile.lastModified / 1000),
            created_at: nowEpoch,
            file_hash: fileHash,
            mime_type: localFile.type || 'application/octet-stream',
            is_directory: false,
          },
          file_content: base64,
        }

        await uploadFile(serverUrl, jwt, payload)
      }

      setInfo(`Uploaded ${selectedFiles.length} file(s).`)
      await fetchFiles(jwt)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed')
    } finally {
      setIsUploading(false)
      event.target.value = ''
    }
  }

  const downloadFile = async (file: ApiFile) => {
    if (!jwt) return
    clearMessages()

    try {
      const payload = await downloadFileContent(serverUrl, jwt, file.file_id)
      const bytes = Uint8Array.from(atob(payload.file_content), (char) => char.charCodeAt(0))
      const blob = new Blob([bytes], { type: payload.mime_type || 'application/octet-stream' })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = url
      anchor.download = payload.file_name || file.file_name
      anchor.click()
      URL.revokeObjectURL(url)
      setInfo(`Downloaded ${payload.file_name}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Download failed')
    }
  }

  const deleteFile = async (file: ApiFile) => {
    if (!jwt) return
    clearMessages()

    try {
      await deleteFileRemote(serverUrl, jwt, file.file_id)
      setInfo(`Deleted ${file.file_name}`)
      await fetchFiles(jwt)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Delete failed')
    }
  }

  const signOut = () => {
    clearCachedAuth()
    setJwt('')
    setUserId('')
    setFiles([])
    setSelectedFolder('')
    setSelectedFileId(null)
    clearMessages()
  }

  return {
    serverUrl,
    setServerUrl,
    jwt,
    userId,
    isAuthenticated,
    info,
    error,
    isFetchingFiles,
    isUploading,
    files,
    folders,
    selectedNode,
    selectedFile,
    selectedFileId,
    setSelectedFileId,
    setSelectedFolder,
    fetchFiles,
    uploadFiles,
    downloadFile,
    deleteFile,
    signOut,
  }
}
