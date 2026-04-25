import { useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { DEFAULT_SERVER_URL } from '../constants/storage'
import { createUser, loginUser } from '../lib/api'
import { clearCachedAuth, getCachedAuth, storeAuth } from '../lib/authStorage'

export const useAuth = () => {
  const navigate = useNavigate()
  const initialAuth = getCachedAuth()

  const [serverUrl, setServerUrl] = useState(initialAuth.serverUrl || DEFAULT_SERVER_URL)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [createUsername, setCreateUsername] = useState('')
  const [createPassword, setCreatePassword] = useState('')
  const [jwt, setJwt] = useState(initialAuth.jwt)
  const [userId, setUserId] = useState(initialAuth.userId)
  const [info, setInfo] = useState('')
  const [error, setError] = useState('')
  const [isAuthenticating, setIsAuthenticating] = useState(false)

  const isAuthenticated = useMemo(() => jwt.length > 0, [jwt])

  const clearMessages = () => {
    setInfo('')
    setError('')
  }

  const login = async (event: React.FormEvent) => {
    event.preventDefault()
    clearMessages()
    setIsAuthenticating(true)

    try {
      const data = await loginUser(serverUrl, username, password)
      setJwt(data.jwt)
      setUserId(data.user_id)
      storeAuth({ jwt: data.jwt, userId: data.user_id, serverUrl })
      setInfo('Logged in successfully.')
      navigate('/dashboard')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setIsAuthenticating(false)
    }
  }

  const register = async (event: React.FormEvent) => {
    event.preventDefault()
    clearMessages()
    setIsAuthenticating(true)

    try {
      await createUser(serverUrl, createUsername, createPassword)
      setInfo('Account created successfully. You can login now.')
      setUsername(createUsername)
      setPassword('')
      setCreateUsername('')
      setCreatePassword('')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not create user')
    } finally {
      setIsAuthenticating(false)
    }
  }

  const signOut = () => {
    clearCachedAuth()
    setJwt('')
    setUserId('')
    setInfo('')
    setError('')
    navigate('/login')
  }

  return {
    serverUrl,
    setServerUrl,
    username,
    setUsername,
    password,
    setPassword,
    createUsername,
    setCreateUsername,
    createPassword,
    setCreatePassword,
    jwt,
    userId,
    isAuthenticated,
    info,
    error,
    isAuthenticating,
    login,
    register,
    signOut,
  }
}
