import { createContext, useContext, useEffect, type ReactNode } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { fetchMe, login as apiLogin, logout as apiLogout } from '../api/auth'
import { setCsrfToken } from '../api/client'
import type { User } from '../types/models'

interface AuthContextValue {
  user: User | null
  isLoading: boolean
  login: (email: string, password: string) => Promise<void>
  logout: () => Promise<void>
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()

  const { data, isLoading } = useQuery({
    queryKey: ['auth', 'me'],
    queryFn: fetchMe,
    retry: false,
    staleTime: Infinity,
  })

  // /auth/me hands back the current session's csrf_token alongside the user,
  // since the token itself only lives in memory (client.ts) and is lost on
  // every page reload - without this, mutations would 403 after a reload
  // until the user logs out and back in.
  useEffect(() => {
    setCsrfToken(data?.csrf_token ?? null)
  }, [data])

  async function login(email: string, password: string) {
    const result = await apiLogin(email, password)
    setCsrfToken(result.csrf_token)
    queryClient.setQueryData(['auth', 'me'], result)
    queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== 'auth' })
  }

  async function logout() {
    await apiLogout()
    setCsrfToken(null)
    queryClient.setQueryData(['auth', 'me'], null)
    queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== 'auth' })
  }

  return (
    <AuthContext.Provider value={{ user: data?.user ?? null, isLoading, login, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return ctx
}
