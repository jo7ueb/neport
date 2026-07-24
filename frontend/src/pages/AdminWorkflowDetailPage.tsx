import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import {
  createStatus,
  createTransition,
  deleteStatus,
  deleteTransition,
  deleteWorkflow,
  getWorkflow,
  updateStatus,
} from '../api/workflows'
import { statusLabel } from '../i18n/statusLabel'
import { Modal } from '../components/Modal'
import { getErrorMessage } from '../i18n/apiErrors'
import type { StatusCategory, WorkflowStatus } from '../types/models'

const CATEGORY_OPTIONS: StatusCategory[] = ['todo', 'in_progress', 'done', 'confirmed']

export function AdminWorkflowDetailPage() {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { workflowId } = useParams()
  const id = Number(workflowId)
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const { data: detail, isLoading } = useQuery({
    queryKey: ['workflow', id],
    queryFn: () => getWorkflow(id),
  })

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['workflow', id] })

  const [error, setError] = useState<string | null>(null)

  const [showCreateStatus, setShowCreateStatus] = useState(false)
  const [newKey, setNewKey] = useState('')
  const [newLabel, setNewLabel] = useState('')
  const [newCategory, setNewCategory] = useState<StatusCategory>('todo')
  const [newIsInitial, setNewIsInitial] = useState(false)

  const createStatusMutation = useMutation({
    mutationFn: () =>
      createStatus(id, {
        key: newKey,
        label: newLabel,
        category: newCategory,
        position: detail?.statuses.length ?? 0,
        is_initial: newIsInitial,
      }),
    onSuccess: () => {
      invalidate()
      setNewKey('')
      setNewLabel('')
      setNewCategory('todo')
      setNewIsInitial(false)
      setError(null)
      setShowCreateStatus(false)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'workflow.statusSaveFailed')),
  })

  const deleteStatusMutation = useMutation({
    mutationFn: (statusId: number) => deleteStatus(id, statusId),
    onSuccess: invalidate,
    onError: (err) => setError(getErrorMessage(err, t, 'workflow.statusDeleteFailed')),
  })

  const [editingStatusId, setEditingStatusId] = useState<number | null>(null)
  const [editLabel, setEditLabel] = useState('')
  const [editCategory, setEditCategory] = useState<StatusCategory>('todo')
  const [editIsInitial, setEditIsInitial] = useState(false)

  const updateStatusMutation = useMutation({
    mutationFn: (statusId: number) =>
      updateStatus(id, statusId, { label: editLabel, category: editCategory, is_initial: editIsInitial }),
    onSuccess: () => {
      invalidate()
      setEditingStatusId(null)
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'workflow.statusSaveFailed')),
  })

  function startEditStatus(status: WorkflowStatus) {
    setEditingStatusId(status.id)
    setEditLabel(status.label)
    setEditCategory(status.category)
    setEditIsInitial(status.is_initial)
  }

  const [fromStatusId, setFromStatusId] = useState<number | ''>('')
  const [toStatusId, setToStatusId] = useState<number | ''>('')

  const createTransitionMutation = useMutation({
    mutationFn: () => createTransition(id, Number(fromStatusId), Number(toStatusId)),
    onSuccess: () => {
      invalidate()
      setFromStatusId('')
      setToStatusId('')
      setError(null)
    },
    onError: (err) => setError(getErrorMessage(err, t, 'workflow.transitionSaveFailed')),
  })

  const deleteTransitionMutation = useMutation({
    mutationFn: (transitionId: number) => deleteTransition(id, transitionId),
    onSuccess: invalidate,
    onError: (err) => setError(getErrorMessage(err, t, 'workflow.transitionDeleteFailed')),
  })

  const deleteWorkflowMutation = useMutation({
    mutationFn: () => deleteWorkflow(id),
    onSuccess: () => navigate('/admin/workflows'),
    onError: (err) => setError(getErrorMessage(err, t, 'workflow.deleteFailed')),
  })

  function handleCreateStatus(e: FormEvent) {
    e.preventDefault()
    createStatusMutation.mutate()
  }

  function handleCreateTransition(e: FormEvent) {
    e.preventDefault()
    if (fromStatusId === '' || toStatusId === '') return
    createTransitionMutation.mutate()
  }

  function statusName(statusId: number): string {
    const status = detail?.statuses.find((s) => s.id === statusId)
    return status ? statusLabel(t, status) : `#${statusId}`
  }

  if (user?.role !== 'admin') {
    return <p className="text-sm text-gray-500">{t('workflow.adminOnly')}</p>
  }

  if (isLoading || !detail) {
    return <p className="text-gray-500">{t('common.loading')}</p>
  }

  return (
    <div>
      <Link to="/admin/workflows" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('workflow.backToList')}
      </Link>

      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">{detail.workflow.name}</h1>
        <button
          onClick={() => window.confirm(t('workflow.confirmDelete')) && deleteWorkflowMutation.mutate()}
          disabled={detail.workflow.is_default || deleteWorkflowMutation.isPending}
          className="rounded border border-red-300 px-3 py-1.5 text-sm text-red-600 hover:bg-red-50 disabled:opacity-30"
        >
          {t('workflow.delete')}
        </button>
      </div>

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      <div className="mb-6 rounded border bg-white p-4">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-lg font-semibold text-gray-800">{t('workflow.statuses')}</h2>
          <button
            onClick={() => setShowCreateStatus(true)}
            className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
          >
            {t('common.new')}
          </button>
        </div>

        <table className="mb-4 w-full text-sm">
          <thead className="border-b text-left text-gray-500">
            <tr>
              <th className="py-1 pr-2">{t('workflow.columnKey')}</th>
              <th className="py-1 pr-2">{t('workflow.columnLabel')}</th>
              <th className="py-1 pr-2">{t('workflow.columnCategory')}</th>
              <th className="py-1 pr-2">{t('workflow.columnInitial')}</th>
              <th className="py-1 pr-2">{t('workflow.columnActions')}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {detail.statuses.map((status) => (
              <tr key={status.id}>
                <td className="py-1 pr-2 font-mono text-xs text-gray-500">{status.key}</td>
                {editingStatusId === status.id ? (
                  <>
                    <td className="py-1 pr-2">
                      <input
                        autoFocus
                        value={editLabel}
                        onChange={(e) => setEditLabel(e.target.value)}
                        className="w-full rounded border border-gray-300 px-1 py-0.5"
                      />
                    </td>
                    <td className="py-1 pr-2">
                      <select
                        value={editCategory}
                        onChange={(e) => setEditCategory(e.target.value as StatusCategory)}
                        className="rounded border border-gray-300 px-1 py-0.5"
                      >
                        {CATEGORY_OPTIONS.map((c) => (
                          <option key={c} value={c}>
                            {t(`workflow.category.${c}`)}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="py-1 pr-2">
                      <input
                        type="checkbox"
                        checked={editIsInitial}
                        onChange={(e) => setEditIsInitial(e.target.checked)}
                      />
                    </td>
                    <td className="py-1 pr-2 whitespace-nowrap">
                      <button
                        onClick={() => updateStatusMutation.mutate(status.id)}
                        className="mr-2 text-blue-600 hover:underline"
                      >
                        {t('common.save')}
                      </button>
                      <button onClick={() => setEditingStatusId(null)} className="text-gray-500 hover:underline">
                        {t('common.cancel')}
                      </button>
                    </td>
                  </>
                ) : (
                  <>
                    <td className="py-1 pr-2">{status.label}</td>
                    <td className="py-1 pr-2">{t(`workflow.category.${status.category}`)}</td>
                    <td className="py-1 pr-2">{status.is_initial ? '★' : ''}</td>
                    <td className="py-1 pr-2 whitespace-nowrap">
                      <button
                        onClick={() => startEditStatus(status)}
                        className="mr-2 text-blue-600 hover:underline"
                      >
                        {t('common.edit')}
                      </button>
                      <button
                        onClick={() => deleteStatusMutation.mutate(status.id)}
                        className="text-red-600 hover:underline"
                      >
                        {t('common.delete')}
                      </button>
                    </td>
                  </>
                )}
              </tr>
            ))}
          </tbody>
        </table>

        <Modal open={showCreateStatus} onClose={() => setShowCreateStatus(false)} title={t('workflow.addStatus')}>
          <form onSubmit={handleCreateStatus} className="space-y-3 text-sm">
            <label className="block">
              <span className="mb-1 block text-gray-600">{t('workflow.columnKey')}</span>
              <input
                required
                value={newKey}
                onChange={(e) => setNewKey(e.target.value)}
                className="w-full rounded border border-gray-300 px-2 py-1"
              />
            </label>
            <label className="block">
              <span className="mb-1 block text-gray-600">{t('workflow.columnLabel')}</span>
              <input
                required
                value={newLabel}
                onChange={(e) => setNewLabel(e.target.value)}
                className="w-full rounded border border-gray-300 px-2 py-1"
              />
            </label>
            <label className="block">
              <span className="mb-1 block text-gray-600">{t('workflow.columnCategory')}</span>
              <select
                value={newCategory}
                onChange={(e) => setNewCategory(e.target.value as StatusCategory)}
                className="w-full rounded border border-gray-300 px-2 py-1"
              >
                {CATEGORY_OPTIONS.map((c) => (
                  <option key={c} value={c}>
                    {t(`workflow.category.${c}`)}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex items-center gap-1">
              <input type="checkbox" checked={newIsInitial} onChange={(e) => setNewIsInitial(e.target.checked)} />
              <span className="text-gray-600">{t('workflow.columnInitial')}</span>
            </label>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setShowCreateStatus(false)}
                className="rounded border border-gray-300 px-3 py-1.5 text-gray-600 hover:bg-gray-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="submit"
                disabled={createStatusMutation.isPending}
                className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
              >
                {t('workflow.addStatus')}
              </button>
            </div>
          </form>
        </Modal>
      </div>

      <div className="rounded border bg-white p-4">
        <h2 className="mb-3 text-lg font-semibold text-gray-800">{t('workflow.transitions')}</h2>

        <ul className="mb-4 space-y-1 text-sm">
          {detail.transitions.map((tr) => (
            <li key={tr.id} className="flex items-center justify-between rounded border bg-gray-50 px-3 py-1.5">
              <span>
                {statusName(tr.from_status_id)} → {statusName(tr.to_status_id)}
              </span>
              <button
                onClick={() => deleteTransitionMutation.mutate(tr.id)}
                className="text-red-600 hover:underline"
              >
                {t('common.delete')}
              </button>
            </li>
          ))}
          {detail.transitions.length === 0 && <li className="text-gray-400">{t('workflow.noTransitions')}</li>}
        </ul>

        <form onSubmit={handleCreateTransition} className="flex flex-wrap items-end gap-2 text-sm">
          <label>
            <span className="mb-1 block text-gray-600">{t('workflow.fromStatus')}</span>
            <select
              value={fromStatusId}
              onChange={(e) => setFromStatusId(e.target.value ? Number(e.target.value) : '')}
              className="rounded border border-gray-300 px-2 py-1"
            >
              <option value="">{t('common.none')}</option>
              {detail.statuses.map((s) => (
                <option key={s.id} value={s.id}>
                  {statusLabel(t, s)}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span className="mb-1 block text-gray-600">{t('workflow.toStatus')}</span>
            <select
              value={toStatusId}
              onChange={(e) => setToStatusId(e.target.value ? Number(e.target.value) : '')}
              className="rounded border border-gray-300 px-2 py-1"
            >
              <option value="">{t('common.none')}</option>
              {detail.statuses.map((s) => (
                <option key={s.id} value={s.id}>
                  {statusLabel(t, s)}
                </option>
              ))}
            </select>
          </label>
          <button
            type="submit"
            disabled={createTransitionMutation.isPending}
            className="rounded bg-blue-600 px-3 py-1.5 text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {t('workflow.addTransition')}
          </button>
        </form>
      </div>
    </div>
  )
}
