import { Fragment, useState, type FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { createTicket, updateTicket, type UpdateTicketInput } from '../api/tickets'
import { statusLabel } from '../i18n/statusLabel'
import { TicketProgress } from './TicketProgress'
import type { Ticket, TicketNode, User, WorkflowDetail, WorkflowStatus } from '../types/models'

interface FlatRow {
  ticket: Ticket
  depth: number
  prevSiblingId: number | null
}

function flatten(nodes: TicketNode[], depth = 0): FlatRow[] {
  const rows: FlatRow[] = []
  let prevSiblingId: number | null = null
  for (const node of nodes) {
    const { children, ...ticket } = node
    rows.push({ ticket, depth, prevSiblingId })
    rows.push(...flatten(children, depth + 1))
    prevSiblingId = node.id
  }
  return rows
}

type EditableField = 'title' | 'status' | 'assignee_id' | 'start_date' | 'end_date' | 'due_date'

export function WbsTable({
  projectId,
  nodes,
  users,
  workflow,
}: {
  projectId: number
  nodes: TicketNode[]
  users: User[]
  workflow?: WorkflowDetail
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['project', projectId, 'tickets', 'tree'] })

  const rows = flatten(nodes)
  const rowsById = new Map(rows.map((r) => [r.ticket.id, r]))

  const [editingCell, setEditingCell] = useState<{ ticketId: number; field: EditableField } | null>(null)
  const [draftValue, setDraftValue] = useState('')
  const [addingChildFor, setAddingChildFor] = useState<number | null>(null)
  const [addingRoot, setAddingRoot] = useState(false)
  const [newTitle, setNewTitle] = useState('')

  const updateMutation = useMutation({
    mutationFn: ({ id, input }: { id: number; input: UpdateTicketInput }) => updateTicket(id, input),
    onSuccess: invalidate,
  })

  const createMutation = useMutation({
    mutationFn: (parentTicketId: number | null) =>
      createTicket({ project_id: projectId, parent_ticket_id: parentTicketId, title: newTitle }),
    onSuccess: () => {
      invalidate()
      setNewTitle('')
      setAddingChildFor(null)
      setAddingRoot(false)
    },
  })

  function startEdit(ticket: Ticket, field: EditableField) {
    if (field === 'status') {
      setDraftValue(String(ticket.status.id))
    } else {
      const raw = ticket[field]
      setDraftValue(raw === null || raw === undefined ? '' : String(raw))
    }
    setEditingCell({ ticketId: ticket.id, field })
  }

  function commitEdit() {
    if (!editingCell) return
    const { ticketId, field } = editingCell
    const input: UpdateTicketInput = {}
    if (field === 'assignee_id') {
      input.assignee_id = draftValue === '' ? null : Number(draftValue)
    } else if (field === 'start_date' || field === 'end_date' || field === 'due_date') {
      input[field] = draftValue === '' ? null : draftValue
    } else if (field === 'status') {
      input.status_id = Number(draftValue)
    } else {
      input.title = draftValue
    }
    updateMutation.mutate({ id: ticketId, input })
    setEditingCell(null)
  }

  // Same "current status + whatever the workflow allows moving to" rule as
  // the ticket detail page's status dropdown.
  function statusOptionsFor(currentStatus: WorkflowStatus): WorkflowStatus[] {
    if (!workflow) return [currentStatus]
    const reachable = new Set(
      workflow.transitions.filter((tr) => tr.from_status_id === currentStatus.id).map((tr) => tr.to_status_id)
    )
    return workflow.statuses.filter((s) => s.id === currentStatus.id || reachable.has(s.id))
  }

  function cancelEdit() {
    setEditingCell(null)
  }

  function indent(row: FlatRow) {
    if (row.prevSiblingId === null) return
    updateMutation.mutate({ id: row.ticket.id, input: { parent_ticket_id: row.prevSiblingId } })
  }

  function outdent(row: FlatRow) {
    if (row.ticket.parent_ticket_id === null) return
    const parentRow = rowsById.get(row.ticket.parent_ticket_id)
    const newParentId = parentRow ? parentRow.ticket.parent_ticket_id : null
    updateMutation.mutate({ id: row.ticket.id, input: { parent_ticket_id: newParentId } })
  }

  function submitNewTask(e: FormEvent, parentTicketId: number | null) {
    e.preventDefault()
    if (!newTitle.trim()) return
    createMutation.mutate(parentTicketId)
  }

  function userLabel(id: number | null): string {
    if (id === null) return t('wbs.unassigned')
    return users.find((u) => u.id === id)?.display_name ?? `#${id}`
  }

  return (
    <div>
      <div className="mb-2">
        {addingRoot ? (
          <form onSubmit={(e) => submitNewTask(e, null)} className="flex items-center gap-2 text-sm">
            <input
              autoFocus
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              placeholder={t('wbs.newTaskPlaceholder')}
              className="rounded border border-gray-300 px-2 py-1"
            />
            <button type="submit" className="rounded bg-blue-600 px-3 py-1 text-white hover:bg-blue-700">
              {t('common.add')}
            </button>
            <button
              type="button"
              onClick={() => {
                setAddingRoot(false)
                setNewTitle('')
              }}
              className="text-gray-500 hover:underline"
            >
              {t('common.cancel')}
            </button>
          </form>
        ) : (
          <button
            onClick={() => setAddingRoot(true)}
            className="rounded border border-gray-300 px-3 py-1 text-sm text-gray-700 hover:bg-gray-100"
          >
            {t('wbs.addRootTask')}
          </button>
        )}
      </div>

      <table className="w-full overflow-hidden rounded border bg-white text-sm">
        <thead className="border-b bg-gray-50 text-left text-gray-500">
          <tr>
            <th className="px-2 py-2">{t('wbs.columns.hierarchy')}</th>
            <th className="px-2 py-2">{t('wbs.columns.title')}</th>
            <th className="px-2 py-2">{t('wbs.columns.status')}</th>
            <th className="px-2 py-2">{t('wbs.columns.progress')}</th>
            <th className="px-2 py-2">{t('wbs.columns.assignee')}</th>
            <th className="px-2 py-2">{t('wbs.columns.startDate')}</th>
            <th className="px-2 py-2">{t('wbs.columns.endDate')}</th>
            <th className="px-2 py-2">{t('wbs.columns.dueDate')}</th>
            <th className="px-2 py-2">{t('wbs.columns.actions')}</th>
          </tr>
        </thead>
        <tbody className="divide-y">
          {rows.map((row) => {
            const { ticket, depth } = row
            const isEditing = (field: EditableField) =>
              editingCell?.ticketId === ticket.id && editingCell.field === field

            return (
              <Fragment key={ticket.id}>
                <tr className="hover:bg-gray-50">
                  <td className="px-2 py-1 whitespace-nowrap">
                    <button
                      onClick={() => outdent(row)}
                      disabled={depth === 0}
                      className="text-gray-400 hover:text-gray-700 disabled:opacity-30"
                      aria-label={t('wbs.outdentAria')}
                    >
                      ←
                    </button>
                    <button
                      onClick={() => indent(row)}
                      disabled={row.prevSiblingId === null}
                      className="text-gray-400 hover:text-gray-700 disabled:opacity-30"
                      aria-label={t('wbs.indentAria')}
                    >
                      →
                    </button>
                  </td>
                  <td className="px-2 py-1" style={{ paddingLeft: 8 + depth * 20 }}>
                    {isEditing('title') ? (
                      <input
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') commitEdit()
                          if (e.key === 'Escape') cancelEdit()
                        }}
                        className="w-full rounded border border-gray-300 px-1 py-0.5"
                      />
                    ) : (
                      <span onClick={() => startEdit(ticket, 'title')} className="cursor-text hover:bg-gray-50">
                        <Link
                          to={`/tickets/${ticket.id}`}
                          onClick={(e) => e.stopPropagation()}
                          className="text-blue-600 hover:underline"
                        >
                          #{ticket.id}
                        </Link>{' '}
                        {ticket.title}
                      </span>
                    )}
                  </td>
                  <td className="px-2 py-1">
                    {isEditing('status') ? (
                      <select
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        className="rounded border border-gray-300 px-1 py-0.5"
                      >
                        {statusOptionsFor(ticket.status).map((s) => (
                          <option key={s.id} value={s.id}>
                            {statusLabel(t, s)}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span onClick={() => startEdit(ticket, 'status')} className="cursor-pointer hover:bg-gray-50">
                        {statusLabel(t, ticket.status)}
                      </span>
                    )}
                  </td>
                  <td className="px-2 py-1">
                    <TicketProgress done={ticket.child_done_ticket_count} total={ticket.child_ticket_count} />
                  </td>
                  <td className="px-2 py-1">
                    {isEditing('assignee_id') ? (
                      <select
                        autoFocus
                        value={draftValue}
                        onChange={(e) => setDraftValue(e.target.value)}
                        onBlur={commitEdit}
                        className="rounded border border-gray-300 px-1 py-0.5"
                      >
                        <option value="">{t('wbs.unassigned')}</option>
                        {users.map((u) => (
                          <option key={u.id} value={u.id}>
                            {u.display_name}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span onClick={() => startEdit(ticket, 'assignee_id')} className="cursor-pointer hover:bg-gray-50">
                        {userLabel(ticket.assignee_id)}
                      </span>
                    )}
                  </td>
                  {(['start_date', 'end_date', 'due_date'] as const).map((field) => (
                    <td key={field} className="px-2 py-1">
                      {isEditing(field) ? (
                        <input
                          autoFocus
                          type="date"
                          value={draftValue}
                          onChange={(e) => setDraftValue(e.target.value)}
                          onBlur={commitEdit}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') commitEdit()
                            if (e.key === 'Escape') cancelEdit()
                          }}
                          className="rounded border border-gray-300 px-1 py-0.5"
                        />
                      ) : (
                        <span onClick={() => startEdit(ticket, field)} className="cursor-pointer hover:bg-gray-50">
                          {ticket[field] ?? '-'}
                        </span>
                      )}
                    </td>
                  ))}
                  <td className="px-2 py-1 whitespace-nowrap">
                    <button
                      onClick={() => setAddingChildFor(ticket.id)}
                      className="text-blue-600 hover:underline"
                    >
                      {t('wbs.addChildTask')}
                    </button>
                  </td>
                </tr>
                {addingChildFor === ticket.id && (
                  <tr key={`${ticket.id}-new`}>
                    <td />
                    <td colSpan={8} style={{ paddingLeft: 8 + (depth + 1) * 20 }} className="px-2 py-1">
                      <form onSubmit={(e) => submitNewTask(e, ticket.id)} className="flex items-center gap-2">
                        <input
                          autoFocus
                          value={newTitle}
                          onChange={(e) => setNewTitle(e.target.value)}
                          placeholder={t('wbs.childTaskPlaceholder')}
                          className="rounded border border-gray-300 px-2 py-1"
                        />
                        <button type="submit" className="rounded bg-blue-600 px-3 py-1 text-white hover:bg-blue-700">
                          {t('common.add')}
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            setAddingChildFor(null)
                            setNewTitle('')
                          }}
                          className="text-gray-500 hover:underline"
                        >
                          {t('common.cancel')}
                        </button>
                      </form>
                    </td>
                  </tr>
                )}
              </Fragment>
            )
          })}
          {rows.length === 0 && (
            <tr>
              <td colSpan={9} className="px-2 py-4 text-center text-gray-400">
                {t('wbs.empty')}
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
