import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import { createUser, listUsers, resetUserPassword, resumeUser, suspendUser, updateUserRole } from '../api/users'
import { Pagination } from '../components/Pagination'
import { UserAvatar } from '../components/UserAvatar'
import { Modal } from '../components/Modal'
import { getErrorMessage } from '../i18n/apiErrors'
import type { UserRole } from '../types/models'

export function AdminUsersPage() {
  const { t } = useTranslation()
  const { user: currentUser } = useAuth()
  const queryClient = useQueryClient()
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useState(25)
  const { data, isLoading } = useQuery({
    queryKey: ['users', page, perPage],
    queryFn: () => listUsers({ page, per_page: perPage }),
  })
  const users = data?.items

  const [showCreate, setShowCreate] = useState(false)
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [role, setRole] = useState<UserRole>('general')
  const [error, setError] = useState<string | null>(null)
  const [resetResult, setResetResult] = useState<{ email: string; password: string } | null>(null)

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['users'] })

  const createMutation = useMutation({
    mutationFn: () => createUser(email, password, displayName, role),
    onSuccess: () => {
      invalidate()
      setEmail('')
      setPassword('')
      setDisplayName('')
      setRole('general')
      setError(null)
      setShowCreate(false)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'adminUsers.createFailed'))
    },
  })

  const suspendMutation = useMutation({ mutationFn: suspendUser, onSuccess: invalidate })
  const resumeMutation = useMutation({ mutationFn: resumeUser, onSuccess: invalidate })
  const resetPasswordMutation = useMutation({
    mutationFn: ({ id }: { id: number; email: string }) => resetUserPassword(id),
    onSuccess: (result, variables) => {
      setResetResult({ email: variables.email, password: result.temporary_password })
    },
  })
  const updateRoleMutation = useMutation({
    mutationFn: ({ id, role }: { id: number; role: UserRole }) => updateUserRole(id, role),
    onSuccess: invalidate,
    onError: (err) => {
      setError(getErrorMessage(err, t, 'adminUsers.roleChangeFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    createMutation.mutate()
  }

  function handleResetPassword(id: number, email: string) {
    if (window.confirm(t('adminUsers.confirmResetPassword', { email }))) {
      resetPasswordMutation.mutate({ id, email })
    }
  }

  function handleSuspend(id: number, email: string) {
    if (window.confirm(t('adminUsers.confirmSuspend', { email }))) {
      suspendMutation.mutate(id)
    }
  }

  if (currentUser?.role !== 'admin') {
    return (
      <div className="rounded border bg-white p-6 text-center text-gray-500">
        {t('adminUsers.adminOnly')}
      </div>
    )
  }

  return (
    <div>
      <Link to="/admin" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('adminHub.backToHub')}
      </Link>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">{t('adminUsers.title')}</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
        >
          {t('common.new')}
        </button>
      </div>

      <Modal open={showCreate} onClose={() => setShowCreate(false)} title={t('adminUsers.title')}>
        <form onSubmit={handleSubmit} className="space-y-3 text-sm">
          {error && <div className="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('adminUsers.email')}</span>
            <input
              type="email"
              required
              autoComplete="username"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('adminUsers.password')}</span>
            <input
              type="password"
              required
              minLength={8}
              autoComplete="new-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('adminUsers.displayName')}</span>
            <input
              type="text"
              required
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-gray-600">{t('adminUsers.role')}</span>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value as UserRole)}
              className="w-full rounded border border-gray-300 px-2 py-1"
            >
              <option value="general">{t('adminUsers.roleGeneral')}</option>
              <option value="admin">{t('adminUsers.roleAdmin')}</option>
            </select>
          </label>
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowCreate(false)}
              className="rounded border border-gray-300 px-3 py-1.5 text-gray-600 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={createMutation.isPending}
              className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('adminUsers.create')}
            </button>
          </div>
        </form>
      </Modal>

      {resetResult && (
        <div className="mb-4 rounded border border-green-300 bg-green-50 px-3 py-2 text-sm text-green-800">
          <p className="mb-1">{t('adminUsers.resetSuccessNotice', { email: resetResult.email })}</p>
          <div className="flex items-center gap-2">
            <code className="rounded bg-white px-2 py-1 font-mono">{resetResult.password}</code>
            <button
              type="button"
              onClick={() => setResetResult(null)}
              className="text-green-700 underline"
            >
              {t('adminUsers.close')}
            </button>
          </div>
        </div>
      )}

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <table className="w-full overflow-hidden rounded border bg-white text-sm">
          <thead className="border-b bg-gray-50 text-left text-gray-500">
            <tr>
              <th className="px-3 py-2">{t('adminUsers.columnDisplayName')}</th>
              <th className="px-3 py-2">{t('adminUsers.columnEmail')}</th>
              <th className="px-3 py-2">{t('adminUsers.columnRole')}</th>
              <th className="px-3 py-2">{t('adminUsers.columnStatus')}</th>
              <th className="px-3 py-2">{t('adminUsers.columnActions')}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {users?.map((u) => (
              <tr key={u.id}>
                <td className="px-3 py-2">
                  <div className="flex items-center gap-2">
                    <UserAvatar user={u} size={24} />
                    {u.display_name}
                  </div>
                </td>
                <td className="px-3 py-2">{u.email}</td>
                <td className="px-3 py-2">
                  {u.id === currentUser?.id ? (
                    u.role === 'admin' ? t('adminUsers.roleAdmin') : t('adminUsers.roleGeneral')
                  ) : (
                    <select
                      value={u.role}
                      disabled={updateRoleMutation.isPending}
                      onChange={(e) => updateRoleMutation.mutate({ id: u.id, role: e.target.value as UserRole })}
                      className="rounded border border-gray-300 px-1 py-0.5"
                    >
                      <option value="general">{t('adminUsers.roleGeneral')}</option>
                      <option value="admin">{t('adminUsers.roleAdmin')}</option>
                    </select>
                  )}
                </td>
                <td className="px-3 py-2">
                  {u.status === 'active' ? t('adminUsers.statusActive') : t('adminUsers.statusSuspended')}
                </td>
                <td className="px-3 py-2 space-x-2">
                  {u.id === currentUser?.id ? (
                    <span className="text-gray-400">{t('adminUsers.selfActionDisabled')}</span>
                  ) : (
                    <>
                      {u.status === 'active' ? (
                        <button
                          onClick={() => handleSuspend(u.id, u.email)}
                          className="text-orange-600 hover:underline"
                        >
                          {t('adminUsers.suspend')}
                        </button>
                      ) : (
                        <button
                          onClick={() => resumeMutation.mutate(u.id)}
                          className="text-green-600 hover:underline"
                        >
                          {t('adminUsers.resume')}
                        </button>
                      )}
                      <button
                        onClick={() => handleResetPassword(u.id, u.email)}
                        className="text-blue-600 hover:underline"
                      >
                        {t('adminUsers.resetPassword')}
                      </button>
                    </>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {data && (
        <Pagination
          page={page}
          perPage={perPage}
          total={data.total}
          onPageChange={setPage}
          onPerPageChange={(n) => {
            setPerPage(n)
            setPage(1)
          }}
        />
      )}
    </div>
  )
}
