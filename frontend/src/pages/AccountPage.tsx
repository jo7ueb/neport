import { useRef, useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import {
  changePassword,
  deleteAvatar,
  uploadAvatar,
  updateProfile,
  getTimezone,
  updateTimezone,
  listApiTokens,
  createApiToken,
  revokeApiToken,
  type LoginResponse,
} from '../api/auth'
import { getErrorMessage } from '../i18n/apiErrors'
import { storePasswordCredential } from '../lib/credentials'
import { useLocaleSync } from '../context/LocaleSyncProvider'
import { useAuth } from '../context/AuthContext'
import { UserAvatar } from '../components/UserAvatar'
import { ALL_TIMEZONES, timezoneOptionLabel } from '../lib/timezones'
import type { Locale, User } from '../types/models'

// Sentinel option value meaning "no personal override - follow the system
// default timezone" (see PATCH /auth/timezone: null clears the override).
const SYSTEM_DEFAULT = ''

export function AccountPage() {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { setLocale } = useLocaleSync()
  const queryClient = useQueryClient()

  function applyUpdatedUser(updated: User) {
    queryClient.setQueryData<LoginResponse | undefined>(['auth', 'me'], (old) =>
      old ? { ...old, user: updated } : old
    )
  }

  const [displayName, setDisplayName] = useState(user?.display_name ?? '')
  const [profileError, setProfileError] = useState<string | null>(null)
  const [profileSuccess, setProfileSuccess] = useState(false)

  const profileMutation = useMutation({
    mutationFn: () => updateProfile(displayName),
    onSuccess: (updated) => {
      applyUpdatedUser(updated)
      setProfileError(null)
      setProfileSuccess(true)
    },
    onError: (err) => {
      setProfileSuccess(false)
      setProfileError(getErrorMessage(err, t, 'account.saveProfileFailed'))
    },
  })

  function handleProfileSubmit(e: FormEvent) {
    e.preventDefault()
    setProfileSuccess(false)
    if (!displayName.trim()) return
    profileMutation.mutate()
  }

  const fileInputRef = useRef<HTMLInputElement>(null)
  const [avatarError, setAvatarError] = useState<string | null>(null)

  const uploadAvatarMutation = useMutation({
    mutationFn: (file: File) => uploadAvatar(file),
    onSuccess: (updated) => {
      applyUpdatedUser(updated)
      setAvatarError(null)
    },
    onError: (err) => {
      setAvatarError(getErrorMessage(err, t, 'account.avatarUploadFailed'))
    },
  })

  const deleteAvatarMutation = useMutation({
    mutationFn: deleteAvatar,
    onSuccess: (updated) => {
      applyUpdatedUser(updated)
      setAvatarError(null)
    },
    onError: (err) => {
      setAvatarError(getErrorMessage(err, t, 'account.avatarDeleteFailed'))
    },
  })

  function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (file) {
      uploadAvatarMutation.mutate(file)
    }
    e.target.value = ''
  }

  const { data: timezoneData } = useQuery({ queryKey: ['auth', 'timezone'], queryFn: getTimezone })
  const [timezoneError, setTimezoneError] = useState<string | null>(null)

  const timezoneMutation = useMutation({
    mutationFn: (value: string) => updateTimezone(value === SYSTEM_DEFAULT ? null : value),
    onSuccess: (result) => {
      queryClient.setQueryData(['auth', 'timezone'], result)
      setTimezoneError(null)
    },
    onError: (err) => {
      setTimezoneError(getErrorMessage(err, t, 'account.saveTimezoneFailed'))
    },
  })

  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  const mutation = useMutation({
    mutationFn: (vars: { current: string; next: string }) => changePassword(vars.current, vars.next),
    onSuccess: (_data, vars) => {
      if (user) void storePasswordCredential(user.email, vars.next)
      setCurrentPassword('')
      setNewPassword('')
      setError(null)
      setSuccess(true)
    },
    onError: (err) => {
      setSuccess(false)
      setError(getErrorMessage(err, t, 'account.changePasswordFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setSuccess(false)
    mutation.mutate({ current: currentPassword, next: newPassword })
  }

  const { data: apiTokens = [], isLoading: tokensLoading } = useQuery({
    queryKey: ['auth', 'tokens'],
    queryFn: listApiTokens,
  })
  const [tokenName, setTokenName] = useState('')
  const [tokenWrite, setTokenWrite] = useState(false)
  const [newToken, setNewToken] = useState<string | null>(null)
  const [tokenError, setTokenError] = useState<string | null>(null)
  const tokenMutation = useMutation({
    mutationFn: () => createApiToken(tokenName.trim(), tokenWrite ? ['read', 'write'] : ['read']),
    onSuccess: (result) => {
      setNewToken(result.token)
      setTokenName('')
      setTokenWrite(false)
      setTokenError(null)
      void queryClient.invalidateQueries({ queryKey: ['auth', 'tokens'] })
    },
    onError: (err) => setTokenError(getErrorMessage(err, t, 'account.tokenCreateFailed')),
  })
  const revokeTokenMutation = useMutation({
    mutationFn: revokeApiToken,
    onSuccess: () => {
      setTokenError(null)
      void queryClient.invalidateQueries({ queryKey: ['auth', 'tokens'] })
    },
    onError: (err) => setTokenError(getErrorMessage(err, t, 'account.tokenRevokeFailed')),
  })

  function handleTokenSubmit(e: FormEvent) {
    e.preventDefault()
    setNewToken(null)
    if (!tokenName.trim()) return
    tokenMutation.mutate()
  }

  if (!user) return null

  return (
    <div>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('account.title')}</h1>

      <div className="mb-6 max-w-sm rounded border bg-white p-4 text-sm">
        <h2 className="mb-3 font-semibold text-gray-800">{t('account.profile')}</h2>

        <div className="mb-4 flex items-center gap-4">
          <UserAvatar user={user} size={64} />
          <div className="flex flex-col gap-1">
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              disabled={uploadAvatarMutation.isPending}
              className="rounded border border-gray-300 px-3 py-1 text-xs hover:bg-gray-50 disabled:opacity-50"
            >
              {t('account.avatarUpload')}
            </button>
            {user.avatar_updated_at && (
              <button
                type="button"
                onClick={() => deleteAvatarMutation.mutate()}
                disabled={deleteAvatarMutation.isPending}
                className="text-xs text-red-600 hover:underline disabled:opacity-50"
              >
                {t('account.avatarRemove')}
              </button>
            )}
            <input
              ref={fileInputRef}
              type="file"
              accept="image/png,image/jpeg,image/webp,image/gif"
              onChange={handleFileChange}
              className="hidden"
            />
          </div>
        </div>
        {avatarError && <div className="mb-3 rounded bg-red-50 px-3 py-2 text-red-700">{avatarError}</div>}

        <form onSubmit={handleProfileSubmit} className="space-y-3">
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('account.displayName')}</span>
            <input
              type="text"
              required
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          {profileError && <div className="rounded bg-red-50 px-3 py-2 text-red-700">{profileError}</div>}
          {profileSuccess && (
            <div className="rounded bg-green-50 px-3 py-2 text-green-700">{t('account.saveProfileSuccess')}</div>
          )}
          <button
            type="submit"
            disabled={profileMutation.isPending}
            className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {t('common.save')}
          </button>
        </form>
      </div>

      <div className="mb-6 max-w-sm rounded border bg-white p-4 text-sm">
        <h2 className="mb-3 font-semibold text-gray-800">{t('account.language')}</h2>
        <select
          value={user.locale}
          onChange={(e) => setLocale(e.target.value as Locale)}
          className="w-full rounded border border-gray-300 px-2 py-1"
        >
          <option value="ja">{t('account.languageJa')}</option>
          <option value="en">{t('account.languageEn')}</option>
        </select>
      </div>

      <div className="mb-6 max-w-sm rounded border bg-white p-4 text-sm">
        <h2 className="mb-3 font-semibold text-gray-800">{t('account.timezone')}</h2>
        {timezoneError && <div className="mb-3 rounded bg-red-50 px-3 py-2 text-red-700">{timezoneError}</div>}
        <select
          value={timezoneData?.timezone ?? SYSTEM_DEFAULT}
          onChange={(e) => timezoneMutation.mutate(e.target.value)}
          disabled={timezoneMutation.isPending}
          className="w-full rounded border border-gray-300 px-2 py-1"
        >
          <option value={SYSTEM_DEFAULT}>
            {t('account.timezoneSystemDefault', {
              timezone: timezoneData?.effective_timezone ? timezoneOptionLabel(timezoneData.effective_timezone) : '',
            })}
          </option>
          {ALL_TIMEZONES.map((tz) => (
            <option key={tz} value={tz}>
              {timezoneOptionLabel(tz)}
            </option>
          ))}
        </select>
        <p className="mt-1 text-xs text-gray-500">{t('account.timezoneDescription')}</p>
      </div>

      <div className="mb-6 max-w-2xl rounded border bg-white p-4 text-sm">
        <h2 className="mb-2 font-semibold text-gray-800">{t('account.apiTokens')}</h2>
        <p className="mb-4 text-xs text-gray-600">{t('account.apiTokensDescription')}</p>
        <form onSubmit={handleTokenSubmit} className="mb-4 space-y-3 rounded bg-gray-50 p-3">
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('account.tokenName')}</span>
            <input
              type="text"
              required
              maxLength={100}
              value={tokenName}
              onChange={(e) => setTokenName(e.target.value)}
              placeholder={t('account.tokenNamePlaceholder')}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="flex items-start gap-2">
            <input type="checkbox" checked={tokenWrite} onChange={(e) => setTokenWrite(e.target.checked)} className="mt-0.5" />
            <span>
              <span className="block text-gray-700">{t('account.tokenWriteScope')}</span>
              <span className="block text-xs text-gray-500">{t('account.tokenWriteScopeDescription')}</span>
            </span>
          </label>
          {tokenError && <div className="rounded bg-red-50 px-3 py-2 text-red-700">{tokenError}</div>}
          <button
            type="submit"
            disabled={tokenMutation.isPending || !tokenName.trim()}
            className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {t('account.createToken')}
          </button>
        </form>

        {newToken && (
          <div className="mb-4 rounded border border-amber-300 bg-amber-50 p-3">
            <p className="font-semibold text-amber-900">{t('account.tokenCreated')}</p>
            <p className="mt-1 text-xs text-amber-800">{t('account.tokenCreatedWarning')}</p>
            <div className="mt-2 flex gap-2">
              <code className="min-w-0 flex-1 break-all rounded bg-white px-2 py-1 text-xs">{newToken}</code>
              <button
                type="button"
                onClick={() => void navigator.clipboard?.writeText(newToken)}
                className="shrink-0 rounded border border-amber-400 px-2 py-1 text-xs text-amber-900 hover:bg-amber-100"
              >
                {t('account.copyToken')}
              </button>
            </div>
          </div>
        )}

        {tokensLoading ? (
          <p className="text-xs text-gray-500">{t('account.tokensLoading')}</p>
        ) : apiTokens.length === 0 ? (
          <p className="text-xs text-gray-500">{t('account.noTokens')}</p>
        ) : (
          <ul className="divide-y rounded border">
            {apiTokens.map((token) => (
              <li key={token.id} className="flex items-center justify-between gap-3 px-3 py-2">
                <div className="min-w-0">
                  <p className="truncate font-medium text-gray-800">{token.name}</p>
                  <p className="text-xs text-gray-500">
                    {token.scopes.join(', ')} · {new Date(token.created_at).toLocaleString()}
                    {token.revoked_at ? ` · ${t('account.tokenRevoked')}` : ''}
                  </p>
                </div>
                {!token.revoked_at && (
                  <button
                    type="button"
                    onClick={() => {
                      if (window.confirm(t('account.revokeTokenConfirm'))) revokeTokenMutation.mutate(token.id)
                    }}
                    disabled={revokeTokenMutation.isPending}
                    className="shrink-0 text-xs text-red-600 hover:underline disabled:opacity-50"
                  >
                    {t('account.revokeToken')}
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <form
        onSubmit={handleSubmit}
        className="max-w-sm space-y-3 rounded border bg-white p-4 text-sm"
      >
        <h2 className="font-semibold text-gray-800">{t('account.changePassword')}</h2>
        {/* Hidden but present so browsers can associate the new password with this account (they otherwise refuse to save a password-only update). */}
        <input type="text" name="username" autoComplete="username" value={user.email} readOnly hidden />
        <label className="block">
          <span className="mb-1 block text-gray-600">{t('account.currentPassword')}</span>
          <input
            type="password"
            name="current-password"
            required
            autoComplete="current-password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            className="w-full rounded border border-gray-300 px-2 py-1"
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-gray-600">{t('account.newPassword')}</span>
          <input
            type="password"
            name="new-password"
            required
            minLength={8}
            autoComplete="new-password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            className="w-full rounded border border-gray-300 px-2 py-1"
          />
        </label>
        {error && <div className="rounded bg-red-50 px-3 py-2 text-red-700">{error}</div>}
        {success && <div className="rounded bg-green-50 px-3 py-2 text-green-700">{t('account.changePasswordSuccess')}</div>}
        <button
          type="submit"
          disabled={mutation.isPending}
          className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('account.submit')}
        </button>
      </form>
    </div>
  )
}
