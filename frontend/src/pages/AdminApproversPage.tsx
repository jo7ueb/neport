import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import {
  addApprover,
  listAllUsers,
  listApprovers,
  listUsers,
  removeApprover,
  searchUsers,
  updateIsApprover,
} from '../api/users'
import { getErrorMessage } from '../i18n/apiErrors'
import { Pagination } from '../components/Pagination'
import type { User } from '../types/models'

function ApproverCell({ user, candidates }: { user: User; candidates: User[] }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [selected, setSelected] = useState<number | ''>('')
  const [error, setError] = useState<string | null>(null)

  const { data: approvers, isLoading } = useQuery({
    queryKey: ['users', user.id, 'approvers'],
    queryFn: () => listApprovers(user.id),
  })

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['users', user.id, 'approvers'] })

  const addMutation = useMutation({
    mutationFn: (approverId: number) => addApprover(user.id, approverId),
    onSuccess: () => {
      invalidate()
      setSelected('')
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'adminApprovers.addFailed')),
  })
  const removeMutation = useMutation({
    mutationFn: (approverId: number) => removeApprover(user.id, approverId),
    onSuccess: invalidate,
  })

  if (isLoading) {
    return <span className="text-xs text-gray-400">{t('common.loading')}</span>
  }

  const availableCandidates = candidates.filter((c) => !approvers?.some((a) => a.id === c.id))

  return (
    <div className="space-y-1">
      <ul className="flex flex-wrap gap-1">
        {approvers?.map((a) => (
          <li key={a.id} className="flex items-center gap-1 rounded bg-gray-100 px-2 py-0.5 text-xs">
            {a.display_name}
            <button
              onClick={() => removeMutation.mutate(a.id)}
              disabled={removeMutation.isPending}
              className="text-red-500 hover:underline"
              aria-label={t('adminApprovers.remove')}
            >
              ×
            </button>
          </li>
        ))}
        {approvers?.length === 0 && <li className="text-xs text-gray-400">{t('adminApprovers.none')}</li>}
      </ul>
      <div className="flex items-center gap-1">
        <select
          value={selected}
          onChange={(e) => setSelected(e.target.value ? Number(e.target.value) : '')}
          className="rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          <option value="">{t('adminApprovers.selectApprover')}</option>
          {availableCandidates.map((c) => (
            <option key={c.id} value={c.id}>
              {c.display_name}
            </option>
          ))}
        </select>
        <button
          onClick={() => selected !== '' && addMutation.mutate(selected)}
          disabled={selected === '' || addMutation.isPending}
          className="rounded bg-blue-600 px-2 py-0.5 text-xs text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('adminApprovers.add')}
        </button>
      </div>
      {error && <p className="text-xs text-red-600">{error}</p>}
    </div>
  )
}

export function AdminApproversPage() {
  const { t } = useTranslation()
  const { user: currentUser } = useAuth()
  const queryClient = useQueryClient()

  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useState(25)
  const [search, setSearch] = useState('')
  const isSearching = search.trim().length > 0

  const { data: page_, isLoading: pageLoading } = useQuery({
    queryKey: ['users', page, perPage],
    queryFn: () => listUsers({ page, per_page: perPage }),
    enabled: !isSearching,
  })
  const { data: searchResults, isLoading: searchLoading } = useQuery({
    queryKey: ['users', 'search', search],
    queryFn: () => searchUsers(search),
    enabled: isSearching,
  })
  const users = isSearching ? searchResults : page_?.items
  const isLoading = isSearching ? searchLoading : pageLoading

  // The "assign as approver" picker needs every approver-flagged user
  // regardless of the current page/search, not just the users currently
  // displayed in the table below.
  const { data: allUsers } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })

  const toggleMutation = useMutation({
    mutationFn: ({ id, isApprover }: { id: number; isApprover: boolean }) => updateIsApprover(id, isApprover),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['users'] })
      // If the admin just toggled their own flag, the cached auth/me data
      // (which drives the nav bar) needs a refresh too - otherwise the
      // "Approvals" nav item won't appear until a full page reload.
      queryClient.invalidateQueries({ queryKey: ['auth', 'me'] })
    },
  })

  if (currentUser?.role !== 'admin') {
    return (
      <div className="rounded border bg-white p-6 text-center text-gray-500">
        {t('adminUsers.adminOnly')}
      </div>
    )
  }

  const candidates = allUsers?.filter((u) => u.is_approver) ?? []

  return (
    <div>
      <Link to="/admin" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('adminHub.backToHub')}
      </Link>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('adminApprovers.title')}</h1>
      <p className="mb-4 text-sm text-gray-500">{t('adminApprovers.description')}</p>

      <input
        type="text"
        value={search}
        onChange={(e) => {
          setSearch(e.target.value)
          setPage(1)
        }}
        placeholder={t('userPicker.placeholder')}
        className="mb-4 w-full max-w-sm rounded border border-gray-300 px-3 py-2 text-sm"
      />

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <table className="w-full overflow-hidden rounded border bg-white text-sm">
          <thead className="border-b bg-gray-50 text-left text-gray-500">
            <tr>
              <th className="px-3 py-2">{t('adminUsers.columnDisplayName')}</th>
              <th className="px-3 py-2">{t('adminApprovers.columnIsApprover')}</th>
              <th className="px-3 py-2">{t('adminApprovers.columnApprovers')}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {users?.map((u) => (
              <tr key={u.id}>
                <td className="px-3 py-2">
                  <div className="font-medium text-gray-800">{u.display_name}</div>
                  <div className="text-xs text-gray-500">{u.email}</div>
                </td>
                <td className="px-3 py-2">
                  <input
                    type="checkbox"
                    checked={u.is_approver}
                    disabled={toggleMutation.isPending}
                    onChange={(e) => toggleMutation.mutate({ id: u.id, isApprover: e.target.checked })}
                  />
                </td>
                <td className="px-3 py-2">
                  <ApproverCell user={u} candidates={candidates} />
                </td>
              </tr>
            ))}
            {users?.length === 0 && (
              <tr>
                <td colSpan={3} className="px-3 py-6 text-center text-gray-400">
                  {t('adminApprovers.noResults')}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      )}

      {!isSearching && page_ && (
        <Pagination
          page={page}
          perPage={perPage}
          total={page_.total}
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
