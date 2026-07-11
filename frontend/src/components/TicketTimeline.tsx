import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { createComment, listComments, listDependencies, listTicketActivity } from '../api/tickets'
import { listAllUsers } from '../api/users'
import { getErrorMessage } from '../i18n/apiErrors'
import { statusLabel } from '../i18n/statusLabel'
import { diffLines } from '../lib/diff'
import { MarkdownContent } from './MarkdownContent'
import { MarkdownField } from './MarkdownField'
import { UserAvatar } from './UserAvatar'
import type { DependentTicket, TicketActivity, TicketComment, WorkflowStatus } from '../types/models'

function isDescriptionUpdate(activity: TicketActivity): boolean {
  return activity.activity_type === 'field_update' && activity.field_name === 'description'
}

function FieldDiff({ from, to }: { from: string | null; to: string | null }) {
  const ops = diffLines(from ?? '', to ?? '')
  return (
    <pre className="mt-1 max-h-64 overflow-auto rounded border bg-white p-2 text-xs whitespace-pre-wrap">
      {ops.map((op, i) => (
        <div
          key={i}
          className={
            op.type === 'add'
              ? 'bg-green-50 text-green-800'
              : op.type === 'remove'
                ? 'bg-red-50 text-red-800'
                : 'text-gray-500'
          }
        >
          {op.type === 'add' ? '+ ' : op.type === 'remove' ? '- ' : '  '}
          {op.line || ' '}
        </div>
      ))}
    </pre>
  )
}

// A single page-sized fetch is enough for one ticket's whole history - no
// dedicated pagination UI, the timeline just shows everything at once.
const TIMELINE_FETCH_SIZE = 200

type TimelineEntry =
  | { kind: 'comment'; at: string; comment: TicketComment }
  | { kind: 'activity'; at: string; activity: TicketActivity }
  | {
      kind: 'dependency'
      at: string
      direction: 'predecessor' | 'successor'
      // A removed dependency keeps its row (see backend DependentTicket
      // .dependency_deleted_at) specifically so this "added" entry doesn't
      // disappear from the timeline - removal gets its own separate entry.
      event: 'added' | 'removed'
      ticket: DependentTicket
    }

export function TicketTimeline({
  ticketId,
  statuses,
}: {
  ticketId: number
  // The project's workflow statuses, already fetched by the ticket detail
  // page - used to resolve status_change from/to ids to labels.
  statuses: WorkflowStatus[]
}) {
  const { t, i18n } = useTranslation()
  const locale = i18n.language === 'ja' ? 'ja-JP' : 'en-US'
  const queryClient = useQueryClient()

  const { data: commentsPage, isLoading: commentsLoading } = useQuery({
    queryKey: ['ticket', ticketId, 'comments', 'timeline'],
    queryFn: () => listComments(ticketId, { page: 1, per_page: TIMELINE_FETCH_SIZE }),
  })
  const { data: activityPage, isLoading: activityLoading } = useQuery({
    queryKey: ['ticket', ticketId, 'activity', 'timeline'],
    queryFn: () => listTicketActivity(ticketId, { page: 1, per_page: TIMELINE_FETCH_SIZE }),
  })
  const { data: dependencies, isLoading: dependenciesLoading } = useQuery({
    queryKey: ['ticket', ticketId, 'dependencies'],
    queryFn: () => listDependencies(ticketId),
  })
  const { data: users } = useQuery({ queryKey: ['users', 'all'], queryFn: listAllUsers })

  function userName(userId: number): string {
    return users?.find((u) => u.id === userId)?.display_name ?? `#${userId}`
  }

  function userById(userId: number) {
    return users?.find((u) => u.id === userId)
  }

  function assigneeName(userId: number | null): string {
    return userId === null ? t('timeline.unassigned') : userName(userId)
  }

  function statusName(statusId: number | null): string | null {
    if (statusId === null) return null
    const status = statuses.find((s) => s.id === statusId)
    return status ? statusLabel(t, status) : null
  }

  const entries: TimelineEntry[] = [
    ...(commentsPage?.items.map((comment): TimelineEntry => ({ kind: 'comment', at: comment.created_at, comment })) ?? []),
    // "comment" activity-log rows exist only so daily reports can see that a
    // ticket was touched - the actual comment content comes from the
    // comments list above, so skip them here to avoid showing it twice.
    ...(activityPage?.items
      .filter((a) => a.activity_type !== 'comment')
      .map((activity): TimelineEntry => ({ kind: 'activity', at: activity.occurred_at, activity })) ?? []),
    ...(dependencies?.predecessors.flatMap((ticket) => dependencyEntries('predecessor', ticket)) ?? []),
    ...(dependencies?.successors.flatMap((ticket) => dependencyEntries('successor', ticket)) ?? []),
  ].sort((a, b) => a.at.localeCompare(b.at))

  function dependencyEntries(direction: 'predecessor' | 'successor', ticket: DependentTicket): TimelineEntry[] {
    const added: TimelineEntry = { kind: 'dependency', at: ticket.dependency_created_at, direction, event: 'added', ticket }
    if (!ticket.dependency_deleted_at) return [added]
    const removed: TimelineEntry = { kind: 'dependency', at: ticket.dependency_deleted_at, direction, event: 'removed', ticket }
    return [added, removed]
  }

  const [body, setBody] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [expandedDiffs, setExpandedDiffs] = useState<Set<number>>(new Set())

  function toggleDiff(activityId: number) {
    setExpandedDiffs((prev) => {
      const next = new Set(prev)
      if (next.has(activityId)) next.delete(activityId)
      else next.add(activityId)
      return next
    })
  }

  const mutation = useMutation({
    mutationFn: () => createComment(ticketId, body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ticket', ticketId, 'comments'] })
      queryClient.invalidateQueries({ queryKey: ['ticket', ticketId, 'activity'] })
      setBody('')
      setError(null)
    },
    onError: (err) => {
      setError(getErrorMessage(err, t, 'comment.postFailed'))
    },
  })

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!body.trim()) return
    mutation.mutate()
  }

  function describeActivity(activity: TicketActivity): string {
    const user = userName(activity.user_id)
    switch (activity.activity_type) {
      case 'status_change': {
        const from = statusName(activity.from_status_id)
        const to = statusName(activity.to_status_id)
        // Rows logged before from/to ids were tracked have neither - fall
        // back to a generic message rather than show "unknown".
        if (!from || !to) return t('timeline.statusChangedGeneric', { user })
        return t('timeline.statusChanged', { user, from, to })
      }
      case 'assignee_change': {
        if (activity.from_assignee_id === null && activity.to_assignee_id === null) {
          return t('timeline.assigneeChangedGeneric', { user })
        }
        return t('timeline.assigneeChanged', {
          user,
          from: assigneeName(activity.from_assignee_id),
          to: assigneeName(activity.to_assignee_id),
        })
      }
      case 'field_update': {
        if (!activity.field_name) return t('timeline.fieldUpdatedGeneric', { user })
        // The description can be long free-form markdown - showing the full
        // before/after inline clutters the timeline, so just say it changed
        // and let the diff button (rendered separately) show the details.
        if (activity.field_name === 'description') return t('timeline.descriptionUpdated', { user })
        const field = t(`timeline.field.${activity.field_name}`, { defaultValue: activity.field_name })
        const from = activity.from_text ?? t('timeline.emptyValue')
        const to = activity.to_text ?? t('timeline.emptyValue')
        return t('timeline.fieldUpdated', { user, field, from, to })
      }
      case 'created':
      default:
        return t('timeline.created', { user })
    }
  }

  const isLoading = commentsLoading || activityLoading || dependenciesLoading

  return (
    <div>
      <h2 className="mb-2 text-lg font-semibold text-gray-800">{t('timeline.title')}</h2>

      {isLoading ? (
        <p className="text-sm text-gray-500">{t('common.loading')}</p>
      ) : (
        <ul className="mb-3 space-y-2">
          {entries.map((entry) => {
            if (entry.kind === 'comment') {
              const author = userById(entry.comment.author_id)
              return (
                <li key={`comment-${entry.comment.id}`} className="rounded border bg-gray-50 px-3 py-2 text-sm">
                  <div className="mb-1 flex items-center gap-2 text-xs font-medium text-gray-600">
                    {author && <UserAvatar user={author} size={20} />}
                    {userName(entry.comment.author_id)}
                  </div>
                  <MarkdownContent text={entry.comment.body} />
                  <div className="mt-1 text-xs text-gray-400">{new Date(entry.comment.created_at).toLocaleString(locale)}</div>
                </li>
              )
            }
            if (entry.kind === 'dependency') {
              const actorId = entry.event === 'added' ? entry.ticket.dependency_created_by_id : entry.ticket.dependency_deleted_by_id
              const dependencyKeys = {
                added: {
                  predecessor: ['timeline.dependencyAddedPredecessor', 'timeline.dependencyAddedPredecessorBy'],
                  successor: ['timeline.dependencyAddedSuccessor', 'timeline.dependencyAddedSuccessorBy'],
                },
                removed: {
                  predecessor: ['timeline.dependencyRemovedPredecessor', 'timeline.dependencyRemovedPredecessorBy'],
                  successor: ['timeline.dependencyRemovedSuccessor', 'timeline.dependencyRemovedSuccessorBy'],
                },
              } as const
              const key = dependencyKeys[entry.event][entry.direction][actorId === null ? 0 : 1]
              return (
                <li key={`dependency-${entry.ticket.dependency_id}-${entry.event}`} className="px-3 py-1 text-xs text-gray-500">
                  {t(key, { user: actorId === null ? undefined : userName(actorId) })}{' '}
                  <Link to={`/tickets/${entry.ticket.id}`} className="text-blue-600 hover:underline">
                    #{entry.ticket.id} {entry.ticket.title}
                  </Link>
                  {' · '}
                  {new Date(entry.at).toLocaleString(locale)}
                </li>
              )
            }
            const showDiffToggle = isDescriptionUpdate(entry.activity)
            const diffOpen = expandedDiffs.has(entry.activity.id)
            return (
              <li key={`activity-${entry.activity.id}`} className="px-3 py-1 text-xs text-gray-500">
                {describeActivity(entry.activity)}
                {' · '}
                {new Date(entry.activity.occurred_at).toLocaleString(locale)}
                {showDiffToggle && (
                  <>
                    {' · '}
                    <button
                      type="button"
                      onClick={() => toggleDiff(entry.activity.id)}
                      className="text-blue-600 hover:underline"
                    >
                      {diffOpen ? t('timeline.hideDiff') : t('timeline.viewDiff')}
                    </button>
                  </>
                )}
                {showDiffToggle && diffOpen && (
                  <FieldDiff from={entry.activity.from_text} to={entry.activity.to_text} />
                )}
              </li>
            )
          })}
          {entries.length === 0 && <li className="text-sm text-gray-400">{t('timeline.empty')}</li>}
        </ul>
      )}

      {error && <div className="mb-2 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}

      <form onSubmit={handleSubmit} className="flex items-end gap-2">
        <MarkdownField
          value={body}
          onChange={setBody}
          placeholder={t('comment.placeholder')}
          rows={2}
          className="flex-1"
        />
        <button
          type="submit"
          disabled={mutation.isPending}
          className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('comment.submit')}
        </button>
      </form>
    </div>
  )
}
