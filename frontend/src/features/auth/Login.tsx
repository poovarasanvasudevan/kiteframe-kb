import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { BookOpen, Eye, EyeOff } from 'lucide-react'
import { Navigate, useNavigate } from 'react-router'
import { Alert, Button, TextField } from '@kiteframe/ui'
import { api, hasSession } from '../../lib/api'

type LoginResponse = {
  access_token: string
  refresh_token: string
  user: { name: string; email: string }
}

export default function Login() {
  const navigate = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const login = useMutation({
    mutationFn: () =>
      api<LoginResponse>('/api/v1/auth/login', {
        method: 'POST',
        body: JSON.stringify({ email, password }),
      }),
    onSuccess: (result) => {
      sessionStorage.setItem('access_token', result.access_token)
      localStorage.setItem('refresh_token', result.refresh_token)
      navigate('/dashboard', { replace: true })
    },
  })

  if (hasSession()) return <Navigate to="/dashboard" replace />

  return (
    <main className="login-page">
      <section className="login-story">
        <div className="login-brand">
          <span>
            <BookOpen />
          </span>
          KiteFrame
        </div>
        <div>
          <span className="eyebrow">Knowledge operations</span>
          <h1>Make every answer accountable.</h1>
          <p>
            One workspace for the people who write, review, publish, and improve organizational knowledge.
          </p>
        </div>
        <small>Tenant-safe · Versioned · Searchable</small>
      </section>

      <section className="login-form-wrap">
        <form
          onSubmit={(event) => {
            event.preventDefault()
            login.mutate()
          }}
        >
          <span className="eyebrow">Welcome back</span>
          <h2>Sign in to your workspace</h2>
          <p>Use the account provided by your organization.</p>

          <TextField
            label="Email address"
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            autoComplete="email"
            required
          />

          <div className="kf-field">
            <span className="kf-field__label">Password</span>
            <div className="password-field">
              <input
                className="kf-input"
                type={showPassword ? 'text' : 'password'}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                autoComplete="current-password"
                minLength={8}
                required
              />
              <Button
                type="button"
                variant="ghost"
                iconOnly
                onClick={() => setShowPassword(!showPassword)}
                aria-label={showPassword ? 'Hide password' : 'Show password'}
              >
                {showPassword ? <EyeOff /> : <Eye />}
              </Button>
            </div>
          </div>

          {login.error ? (
            <Alert tone="danger" role="alert">
              {login.error.message}
            </Alert>
          ) : null}

          <Button
            type="submit"
            variant="primary"
            className="login-submit"
            disabled={login.isPending}
            style={{ width: '100%' }}
          >
            {login.isPending ? 'Signing in…' : 'Sign in'}
          </Button>

          <a href="mailto:workspace-admin@example.invalid">
            Need access? Contact your workspace administrator.
          </a>
        </form>
      </section>
    </main>
  )
}
