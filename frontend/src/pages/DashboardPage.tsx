import {
  ArrowDownToLine,
  ChevronRight,
  CloudUpload,
  FolderClosed,
  FolderOpen,
  HardDrive,
  LayoutGrid,
  ListFilter,
  LogOut,
  PanelLeft,
  RefreshCw,
  Search,
  Server,
  Trash2,
  UserRound,
} from 'lucide-react'
import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useDashboard } from '../hooks/useDashboard'
import { formatBytes } from '../lib/fileTree'

const formatDate = (timestamp: number) =>
  new Date(timestamp * 1000).toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })

export const DashboardPage = () => {
  const navigate = useNavigate()
  const {
    serverUrl,
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
  } = useDashboard()

  useEffect(() => {
    if (!isAuthenticated) {
      navigate('/login', { replace: true })
      return
    }
    void fetchFiles(jwt)
  }, [fetchFiles, isAuthenticated, jwt, navigate])

  if (!isAuthenticated) {
    return null
  }

  return (
    <section className="explorer-shell overflow-hidden rounded-2xl border border-white/10">
      <div className="explorer-titlebar flex items-center justify-between px-4 py-2.5">
        <div className="flex items-center gap-2 text-sm text-zinc-200">
          <PanelLeft size={15} />
          <span className="font-medium">File Explorer</span>
          <ChevronRight size={12} className="text-zinc-500" />
          <span className="text-zinc-400">Anywhere Door Cloud</span>
        </div>
        <button type="button" onClick={signOut} className="explorer-toolbar-btn">
          <LogOut size={14} />
          Sign out
        </button>
      </div>

      <div className="explorer-toolbar flex flex-wrap items-center justify-between gap-2 px-4 py-2">
        <div className="flex flex-wrap items-center gap-2">
          <label className="explorer-toolbar-primary cursor-pointer">
            <CloudUpload size={14} />
            {isUploading ? 'Uploading...' : 'Upload'}
            <input
              type="file"
              multiple
              disabled={!jwt || isUploading}
              className="hidden"
              onChange={(event) => {
                void uploadFiles(event)
              }}
            />
          </label>

          <button
            type="button"
            onClick={() => selectedFile && void downloadFile(selectedFile)}
            disabled={!selectedFile}
            className="explorer-toolbar-btn"
          >
            <ArrowDownToLine size={14} /> Download
          </button>
          <button
            type="button"
            onClick={() => selectedFile && void deleteFile(selectedFile)}
            disabled={!selectedFile}
            className="explorer-toolbar-btn"
          >
            <Trash2 size={14} /> Delete
          </button>
          <button
            type="button"
            disabled={isFetchingFiles}
            onClick={() => void fetchFiles()}
            className="explorer-toolbar-btn"
          >
            <RefreshCw size={14} className={isFetchingFiles ? 'animate-spin' : ''} />
            Refresh
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button type="button" className="explorer-toolbar-btn">
            <ListFilter size={14} /> Sort
          </button>
          <button type="button" className="explorer-toolbar-btn">
            <LayoutGrid size={14} /> View
          </button>
          <div className="explorer-search">
            <Search size={14} className="text-zinc-500" />
            <span className="text-xs text-zinc-500">Search files</span>
          </div>
        </div>
      </div>

      <div className="grid min-h-[620px] grid-cols-[240px_1fr]">
        <aside className="explorer-sidebar border-r border-white/10 p-3">
          <div className="mb-4 space-y-1 text-xs">
            <p className="explorer-side-meta">
              <Server size={13} /> {serverUrl}
            </p>
            <p className="explorer-side-meta">
              <UserRound size={13} /> {userId}
            </p>
            <p className="explorer-side-meta">
              <HardDrive size={13} /> {files.length} files
            </p>
          </div>

          <p className="mb-2 px-2 text-[11px] uppercase tracking-[0.1em] text-zinc-500">Folders</p>
          <div className="space-y-0.5">
            {folders.map((folder) => {
              const depth = folder.path ? folder.path.split('/').length - 1 : 0
              const isSelected = folder.path === selectedNode.path
              return (
                <button
                  key={folder.path || 'root'}
                  type="button"
                  onClick={() => {
                    setSelectedFolder(folder.path)
                    setSelectedFileId(folder.files[0]?.file_id ?? null)
                  }}
                  className={`explorer-folder ${isSelected ? 'explorer-folder-active' : ''}`}
                  style={{ paddingLeft: `${10 + depth * 12}px` }}
                >
                  {isSelected ? <FolderOpen size={13} /> : <FolderClosed size={13} />}
                  <span className="truncate">{folder.path || 'root'}</span>
                </button>
              )
            })}
          </div>
        </aside>

        <section className="explorer-content">
          <div className="explorer-table-head grid grid-cols-[3fr_1.4fr_1fr_0.8fr] px-4 py-2 text-xs text-zinc-400">
            <span>Name</span>
            <span>Date modified</span>
            <span>Type</span>
            <span className="text-right">Size</span>
          </div>

          <div className="max-h-[546px] overflow-y-auto">
            {selectedNode.files.length === 0 ? (
              <div className="p-8 text-center text-sm text-zinc-500">This folder does not contain files.</div>
            ) : (
              selectedNode.files
                .slice()
                .sort((a, b) => b.uploaded_at - a.uploaded_at)
                .map((file) => {
                  const isSelected = selectedFileId === file.file_id
                  return (
                    <button
                      key={file.file_id}
                      type="button"
                      onClick={() => setSelectedFileId(file.file_id)}
                      className={`explorer-row grid w-full grid-cols-[3fr_1.4fr_1fr_0.8fr] px-4 py-2.5 text-left ${
                        isSelected ? 'explorer-row-active' : ''
                      }`}
                    >
                      <span className="truncate font-medium text-zinc-100">{file.file_name}</span>
                      <span className="truncate text-zinc-400">{formatDate(file.uploaded_at)}</span>
                      <span className="truncate text-zinc-400">{file.mime_type || 'File'}</span>
                      <span className="text-right text-zinc-400">{formatBytes(file.file_size)}</span>
                    </button>
                  )
                })
            )}
          </div>

          <div className="explorer-statusbar flex flex-wrap items-center justify-between gap-2 px-4 py-2 text-xs">
            <span>{selectedNode.files.length} item(s) in {selectedNode.path || 'root'}</span>
            <span className="truncate">{selectedFile ? selectedFile.file_path : 'No file selected'}</span>
          </div>
        </section>
      </div>

      {info ? <p className="explorer-info border-t border-white/10 px-4 py-2">{info}</p> : null}
      {error ? <p className="explorer-error border-t border-white/10 px-4 py-2">{error}</p> : null}
    </section>
  )
}
