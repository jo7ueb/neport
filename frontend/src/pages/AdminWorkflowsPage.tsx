import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import { createWorkflow, listWorkflows } from '../api/workflows'
import { Modal } from '../components/Modal'
import { getErrorMessage } from '../i18n/apiErrors'

export function AdminWorkflowsPage() {
  const { t } = useTranslation()
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const { data: workflows, isLoading } = useQuery({ queryKey: ['workflows'], queryFn: listWorkflows })

  const [showCreate, setShowCreate] = useState(false)
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)

  const createMutation = useMutation({
    mutationFn: () => createWorkflow(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workflows'] })
      setName('')
      setError(null)
      setShowCreate(false)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'workflow.createFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!name.trim()) return
    createMutation.mutate()
  }

  if (user?.role !== 'admin') {
    return <p className="text-sm text-gray-500">{t('workflow.adminOnly')}</p>
  }

  return (
    <div>
      <Link to="/admin" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('adminHub.backToHub')}
      </Link>
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-gray-800">{t('workflow.title')}</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
        >
          {t('common.new')}
        </button>
      </div>

      <Modal open={showCreate} onClose={() => setShowCreate(false)} title={t('workflow.title')}>
        <form onSubmit={handleSubmit} className="space-y-3">
          {error && <div className="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
          <input
            type="text"
            required
            placeholder={t('workflow.namePlaceholder')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full rounded border border-gray-300 px-3 py-2"
          />
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowCreate(false)}
              className="rounded border border-gray-300 px-4 py-2 text-gray-600 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={createMutation.isPending}
              className="rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('common.create')}
            </button>
          </div>
        </form>
      </Modal>

      {isLoading ? (
        <p className="text-gray-500">{t('common.loading')}</p>
      ) : (
        <ul className="divide-y rounded border bg-white">
          {workflows?.map((workflow) => (
            <li key={workflow.id}>
              <Link
                to={`/admin/workflows/${workflow.id}`}
                className="flex items-center justify-between px-4 py-3 hover:bg-gray-50"
              >
                <span className="font-medium text-gray-800">{workflow.name}</span>
                {workflow.is_default && (
                  <span className="rounded bg-blue-100 px-2 py-0.5 text-xs text-blue-700">
                    {t('workflow.defaultBadge')}
                  </span>
                )}
              </Link>
            </li>
          ))}
          {workflows?.length === 0 && (
            <li className="px-4 py-6 text-center text-gray-400">{t('workflow.empty')}</li>
          )}
        </ul>
      )}
    </div>
  )
}
