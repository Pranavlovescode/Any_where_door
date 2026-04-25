import type { ApiFile, ExplorerNode } from '../types/models'

export const formatBytes = (size: number): string => {
  if (size < 1024) return `${size} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = size / 1024
  let idx = 0

  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024
    idx += 1
  }

  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[idx]}`
}

export const normalizeFileSegments = (file: ApiFile): string[] => {
  const rawPath = (file.file_path || '').replace(/\\/g, '/').trim()
  const cleanPath = rawPath.replace(/^\/+|\/+$/g, '')
  const baseSegments = cleanPath ? cleanPath.split('/').filter(Boolean) : []

  if (baseSegments.at(-1) !== file.file_name) {
    baseSegments.push(file.file_name)
  }

  return baseSegments
}

export const buildTree = (files: ApiFile[]): ExplorerNode => {
  const root: ExplorerNode = {
    name: 'root',
    path: '',
    files: [],
    folders: new Map(),
  }

  files.forEach((file) => {
    const segments = normalizeFileSegments(file)
    if (segments.length === 0) {
      root.files.push(file)
      return
    }

    const fileName = segments.at(-1)
    if (!fileName) {
      root.files.push(file)
      return
    }

    let current = root
    const directorySegments = segments.slice(0, -1)

    directorySegments.forEach((segment) => {
      const currentPath = [current.path, segment].filter(Boolean).join('/')
      if (!current.folders.has(segment)) {
        current.folders.set(segment, {
          name: segment,
          path: currentPath,
          files: [],
          folders: new Map(),
        })
      }
      current = current.folders.get(segment) as ExplorerNode
    })

    current.files.push(file)
  })

  return root
}

export const flattenFolders = (node: ExplorerNode, out: ExplorerNode[] = []): ExplorerNode[] => {
  out.push(node)
  ;[...node.folders.values()]
    .sort((a, b) => a.name.localeCompare(b.name))
    .forEach((child) => flattenFolders(child, out))
  return out
}
