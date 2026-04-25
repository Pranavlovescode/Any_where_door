import { useCallback, useEffect, useRef, useState } from 'react'
import { getCachedAuth } from '../lib/authStorage'

export type FileEventType = 'upload' | 'delete' | 'update' | 'scan'

export type FileEvent = {
  type: 'file_event'
  event_type: FileEventType
  file_id: string
  file_name: string
  file_path: string
  file_size: number
  source: 'agent' | 'frontend'
  timestamp: number
}

export type SyncStatus = 'syncing' | 'synced' | 'error'

export type SyncStatusEvent = {
  type: 'sync_status'
  status: SyncStatus
  file_count: number
  message: string
  timestamp: number
}

export type SyncEvent = FileEvent | SyncStatusEvent

export const useFileSync = (serverUrl: string, onFileEvent?: (event: FileEvent) => void, onSyncStatus?: (event: SyncStatusEvent) => void) => {
  const [isConnected, setIsConnected] = useState(false)
  const [lastEvent, setLastEvent] = useState<SyncEvent | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const heartbeatTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Send heartbeat to keep connection alive
  const sendHeartbeat = useCallback((ws: WebSocket) => {
    if (heartbeatTimeoutRef.current) {
      clearTimeout(heartbeatTimeoutRef.current)
    }

    try {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send('ping')
      }
    } catch (err) {
      console.warn('[FileSync] Failed to send heartbeat:', err)
    }

    // Send another heartbeat every 30 seconds
    heartbeatTimeoutRef.current = setTimeout(() => {
      sendHeartbeat(ws)
    }, 30000)
  }, [])

  const connect = useCallback(() => {
    const auth = getCachedAuth()
    if (!auth.jwt) {
      console.warn('useFileSync: No JWT available, cannot connect')
      return
    }

    // Construct WebSocket URL from serverUrl
    const wsBaseUrl = serverUrl.replace(/^http/, 'ws')
    const wsUrl = `${wsBaseUrl}/ws/sync/${encodeURIComponent(auth.jwt)}`

    console.log('[FileSync] Connecting to WebSocket:', wsUrl.replace(auth.jwt, '***'))

    try {
      const ws = new WebSocket(wsUrl)

      ws.onopen = () => {
        console.log('[FileSync] WebSocket connected')
        setIsConnected(true)
        // Clear any pending reconnect timeout
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current)
          reconnectTimeoutRef.current = null
        }
        // Start heartbeat
        sendHeartbeat(ws)
      }

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as SyncEvent
          console.log('[FileSync] Received event:', message.type, message)
          setLastEvent(message)

          if (message.type === 'file_event') {
            onFileEvent?.(message as FileEvent)
          } else if (message.type === 'sync_status') {
            onSyncStatus?.(message as SyncStatusEvent)
          }
        } catch (err) {
          console.error('[FileSync] Failed to parse message:', err)
        }
      }

      ws.onerror = (error) => {
        console.error('[FileSync] WebSocket error:', error)
        setIsConnected(false)
      }

      ws.onclose = () => {
        console.log('[FileSync] WebSocket closed, attempting reconnect in 3s...')
        setIsConnected(false)
        // Attempt reconnect after 3 seconds
        reconnectTimeoutRef.current = setTimeout(() => {
          connect()
        }, 3000)
      }

      wsRef.current = ws
    } catch (err) {
      console.error('[FileSync] Failed to create WebSocket:', err)
      setIsConnected(false)
    }
  }, [onFileEvent, onSyncStatus, sendHeartbeat])

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close()
      wsRef.current = null
    }
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
    if (heartbeatTimeoutRef.current) {
      clearTimeout(heartbeatTimeoutRef.current)
      heartbeatTimeoutRef.current = null
    }
    setIsConnected(false)
  }, [])

  // Auto-connect on mount
  useEffect(() => {
    connect()

    return () => {
      disconnect()
    }
  }, [connect, disconnect])

  return {
    isConnected,
    lastEvent,
    disconnect,
    reconnect: connect,
  }
}
