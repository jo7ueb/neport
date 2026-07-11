import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { createDependency, deleteDependency, listDependencies } from '../api/tickets'
import { getErrorMessage } from '../i18n/apiErrors'
import { TicketPicker } from './TicketPicker'
import type { Ticket } from '../types/models'

export function DependencyList({ ticketId }: { ticketId: number }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { data: rawData, isLoading } = useQuery({
    queryKey: ['ticket', ticketId, 'dependencies'],
    queryFn: () => listDependencies(ticketId),
  })
  // The endpoint also returns removed links (kept around so the timeline can
  // still show "X removed this dependency") - this list only shows the
  // currently active ones.
  const data = rawData && {
    predecessors: rawData.predecessors.filter((t) => !t.dependency_deleted_at),
    successors: rawData.successors.filter((t) => !t.dependency_deleted_at),
  }

  const [direction, setDirection] = useState<'predecessor' | 'successor'>('successor')
  const [otherTicket, setOtherTicket] = useState<Ticket | null>(null)
  const [error, setError] = useState<string | null>(null)

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ['ticket', ticketId, 'dependencies'] })

  const createMutation = useMutation({
    mutationFn: () => createDependency(ticketId, direction, otherTicket!.id),
    onSuccess: () => {
      invalidate()
      setOtherTicket(null)
      setError(null)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'dependency.createFailed'))
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (dependencyId: number) => deleteDependency(ticketId, dependencyId),
    onSuccess: invalidate,
    onError: (err) => {
      setError(getErrorMessage(err, t, 'dependency.deleteFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!otherTicket) return
    createMutation.mutate()
  }

  return (
    <div>
      <h2 className="mb-2 text-lg font-semibold text-gray-800">{t('dependency.title')}</h2>

      {isLoading ? (
        <p className="text-sm text-gray-500">{t('common.loading')}</p>
      ) : (
        <div className="mb-3 space-y-3 text-sm">
          <div>
            <div className="mb-1 text-gray-500">{t('dependency.predecessors')}</div>
            {data?.predecessors.length ? (
              <ul className="space-y-1">
                {data.predecessors.map((t) => (
                  <li key={t.dependency_id} className="flex items-center justify-between rounded border px-2 py-1">
                    <Link to={`/tickets/${t.id}`} className="text-blue-600 hover:underline">
                      #{t.id} {t.title}
                    </Link>
                    <DeleteDependencyButton onClick={() => deleteMutation.mutate(t.dependency_id)} />
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-gray-400">{t('dependency.none')}</p>
            )}
          </div>
          <div>
            <div className="mb-1 text-gray-500">{t('dependency.successors')}</div>
            {data?.successors.length ? (
              <ul className="space-y-1">
                {data.successors.map((t) => (
                  <li key={t.dependency_id} className="flex items-center justify-between rounded border px-2 py-1">
                    <Link to={`/tickets/${t.id}`} className="text-blue-600 hover:underline">
                      #{t.id} {t.title}
                    </Link>
                    <DeleteDependencyButton onClick={() => deleteMutation.mutate(t.dependency_id)} />
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-gray-400">{t('dependency.none')}</p>
            )}
          </div>
        </div>
      )}

      {error && <div className="mb-2 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      <form onSubmit={handleSubmit} className="flex items-center gap-2 text-sm">
        <select
          value={direction}
          onChange={(e) => setDirection(e.target.value as 'predecessor' | 'successor')}
          className="rounded border border-gray-300 px-2 py-1"
        >
          <option value="predecessor">{t('dependency.addAsPredecessor')}</option>
          <option value="successor">{t('dependency.addAsSuccessor')}</option>
        </select>
        <TicketPicker
          value={otherTicket}
          onChange={setOtherTicket}
          placeholder={t('dependency.idPlaceholder')}
          excludeId={ticketId}
          className="w-56"
        />
        <button
          type="submit"
          disabled={createMutation.isPending || !otherTicket}
          className="rounded bg-blue-600 px-3 py-1 text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('dependency.add')}
        </button>
      </form>
    </div>
  )
}

function DeleteDependencyButton({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation()
  return (
    <button onClick={onClick} className="text-red-600 hover:underline">
      {t('dependency.delete')}
    </button>
  )
}
