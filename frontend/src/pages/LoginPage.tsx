import { LoaderCircle, LogIn, UserPlus } from 'lucide-react'
import { useAuth } from '../hooks/useAuth'

export const LoginPage = () => {
  const {
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
    info,
    error,
    isAuthenticating,
    login,
    register,
  } = useAuth()

  return (
    <section className="mx-auto grid max-w-6xl gap-4 md:grid-cols-2">
      <article className="explorer-shell rounded-2xl border border-white/10 p-6">
        <p className="mb-2 text-xs uppercase tracking-[0.12em] text-emerald-300">Anywhere Door</p>
        <h1 className="font-display text-3xl text-white">Secure Agent Login</h1>
        <p className="mt-2 text-sm text-white/65">
          Sign in with your backend credentials to open the virtual file explorer dashboard.
        </p>

        <form onSubmit={login} className="mt-6 space-y-3">
          <label className="explorer-label">
            Backend URL
            <input
              value={serverUrl}
              onChange={(event) => setServerUrl(event.target.value)}
              className="explorer-input"
              placeholder="http://127.0.0.1:8000"
            />
          </label>
          <label className="explorer-label">
            Username
            <input
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              className="explorer-input"
              placeholder="testuser"
            />
          </label>
          <label className="explorer-label">
            Password
            <input
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              className="explorer-input"
              placeholder="********"
            />
          </label>

          <button type="submit" disabled={isAuthenticating} className="explorer-primary-btn">
            {isAuthenticating ? <LoaderCircle className="animate-spin" size={15} /> : <LogIn size={15} />}
            Login
          </button>
        </form>
      </article>

      <article className="explorer-shell rounded-2xl border border-white/10 p-6">
        <p className="mb-2 text-xs uppercase tracking-[0.12em] text-cyan-300">First Time Setup</p>
        <h2 className="font-display text-2xl text-white">Create Account</h2>
        <p className="mt-2 text-sm text-white/65">
          Quickly create a backend account for local testing and dashboard access.
        </p>

        <form onSubmit={register} className="mt-6 space-y-3">
          <label className="explorer-label">
            New Username
            <input
              value={createUsername}
              onChange={(event) => setCreateUsername(event.target.value)}
              className="explorer-input"
              placeholder="new-user"
            />
          </label>
          <label className="explorer-label">
            New Password
            <input
              type="password"
              value={createPassword}
              onChange={(event) => setCreatePassword(event.target.value)}
              className="explorer-input"
              placeholder="strong-password"
            />
          </label>

          <button type="submit" disabled={isAuthenticating} className="explorer-secondary-btn">
            {isAuthenticating ? (
              <LoaderCircle className="animate-spin" size={15} />
            ) : (
              <UserPlus size={15} />
            )}
            Create User
          </button>
        </form>
      </article>

      {info ? <p className="explorer-info md:col-span-2">{info}</p> : null}
      {error ? <p className="explorer-error md:col-span-2">{error}</p> : null}
    </section>
  )
}
