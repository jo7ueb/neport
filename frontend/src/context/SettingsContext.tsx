import { createContext, useContext, type ReactNode } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { getSettings, updateSettings } from '../api/settings'
import { useAuth } from './AuthContext'

interface SettingsContextValue {
  katexEnabled: boolean
  defaultTimezone: string
  maintenanceMode: boolean
  updateSettings: (katexEnabled: boolean, defaultTimezone: string, maintenanceMode: boolean) => Promise<void>
  isUpdating: boolean
}

const SettingsContext = createContext<SettingsContextValue | null>(null)

export function SettingsProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth()
  const queryClient = useQueryClient()

  const { data } = useQuery({
    queryKey: ['settings'],
    queryFn: getSettings,
    enabled: !!user,
    staleTime: Infinity,
    // maintenance_mode can be flipped by an admin at any time, so this one
    // field needs to reach other open sessions reasonably promptly - the
    // same polling interval used for the notification unread count.
    refetchInterval: 30_000,
  })

  const mutation = useMutation({
    mutationFn: updateSettings,
    onSuccess: (result) => {
      queryClient.setQueryData(['settings'], result)
    },
  })

  async function updateSettingsFields(katexEnabled: boolean, defaultTimezone: string, maintenanceMode: boolean) {
    await mutation.mutateAsync({ katex_enabled: katexEnabled, default_timezone: defaultTimezone, maintenance_mode: maintenanceMode })
  }

  return (
    <SettingsContext.Provider
      value={{
        katexEnabled: data?.katex_enabled ?? true,
        defaultTimezone: data?.default_timezone ?? 'Asia/Tokyo',
        maintenanceMode: data?.maintenance_mode ?? false,
        updateSettings: updateSettingsFields,
        isUpdating: mutation.isPending,
      }}
    >
      {children}
    </SettingsContext.Provider>
  )
}

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext)
  if (!ctx) {
    throw new Error('useSettings must be used within a SettingsProvider')
  }
  return ctx
}
