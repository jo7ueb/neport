import { createContext, useContext, useEffect, type ReactNode } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { updateLocale } from '../api/auth'
import { useAuth } from './AuthContext'
import type { Locale } from '../types/models'

interface LocaleSyncContextValue {
  setLocale: (locale: Locale) => void
}

const LocaleSyncContext = createContext<LocaleSyncContextValue | null>(null)

export function LocaleSyncProvider({ children }: { children: ReactNode }) {
  const { i18n } = useTranslation()
  const { user } = useAuth()
  const queryClient = useQueryClient()

  // The DB-stored locale (attached to the logged-in user) is the source of
  // truth so a user's language choice follows them across devices/sessions.
  // Logged-out screens (e.g. the login page) keep i18next's initial 'ja'.
  useEffect(() => {
    if (user?.locale && user.locale !== i18n.language) {
      i18n.changeLanguage(user.locale)
    }
  }, [user?.locale, i18n])

  const mutation = useMutation({
    mutationFn: (locale: Locale) => updateLocale(locale),
    onSuccess: (_data, locale) => {
      queryClient.setQueryData(['auth', 'me'], (prev: { user: { locale: Locale } } | null | undefined) =>
        prev ? { ...prev, user: { ...prev.user, locale } } : prev
      )
    },
  })

  function setLocale(locale: Locale) {
    i18n.changeLanguage(locale)
    mutation.mutate(locale)
  }

  return <LocaleSyncContext.Provider value={{ setLocale }}>{children}</LocaleSyncContext.Provider>
}

export function useLocaleSync(): LocaleSyncContextValue {
  const ctx = useContext(LocaleSyncContext)
  if (!ctx) {
    throw new Error('useLocaleSync must be used within a LocaleSyncProvider')
  }
  return ctx
}
