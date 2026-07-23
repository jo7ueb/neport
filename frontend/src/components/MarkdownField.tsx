import { useRef, useState, type ChangeEvent, type ClipboardEvent, type DragEvent, type KeyboardEvent } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { uploadAttachment } from '../api/attachments'
import { searchTickets } from '../api/tickets'
import { searchUsers } from '../api/users'
import { getErrorMessage } from '../i18n/apiErrors'
import type { Ticket, User } from '../types/models'
import { MarkdownContent } from './MarkdownContent'

const ALLOWED_IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif']

let placeholderCounter = 0

interface MarkdownFieldProps {
  value: string
  onChange: (value: string) => void
  rows?: number
  placeholder?: string
  className?: string
  // Whether this field should open already showing the rendered preview
  // rather than the raw textarea. Only affects the initial mount - use a
  // `key` on the caller's side if the underlying record can change (e.g.
  // navigating from one ticket to another) so this is re-evaluated.
  startInPreview?: boolean
}

// Tracks an in-progress "#123" ticket-ref or "@name" user-mention being
// typed: `start` is the index of the trigger character ("#"/"@") so the
// picked item can be spliced back in at the right spot.
interface MentionState {
  kind: 'ticket' | 'user'
  query: string
  start: number
}

export function MarkdownField({
  value,
  onChange,
  rows = 3,
  placeholder,
  className,
  startInPreview = false,
}: MarkdownFieldProps) {
  const { t } = useTranslation()
  const [preview, setPreview] = useState(startInPreview)
  const [mention, setMention] = useState<MentionState | null>(null)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [uploadError, setUploadError] = useState<string | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  // Mirrors `value` so async upload completions can splice into the latest
  // text even if the user kept typing while the upload was in flight -
  // `value` itself would be stale inside that already-started closure.
  const valueRef = useRef(value)
  valueRef.current = value

  const { data: ticketResults } = useQuery({
    queryKey: ['tickets', 'search', mention?.query],
    queryFn: () => searchTickets(mention!.query),
    enabled: mention?.kind === 'ticket',
    staleTime: 30_000,
  })
  const { data: userResults } = useQuery({
    queryKey: ['users', 'search', mention?.query],
    queryFn: () => searchUsers(mention!.query),
    enabled: mention?.kind === 'user',
    staleTime: 30_000,
  })
  const mentionResults = mention?.kind === 'user' ? userResults : ticketResults

  function handleChange(e: ChangeEvent<HTMLTextAreaElement>) {
    const newValue = e.target.value
    onChange(newValue)

    const caret = e.target.selectionStart
    const upToCaret = newValue.slice(0, caret)
    const ticketMatch = /#(\d*)$/.exec(upToCaret)
    // Requires the "@" to be at the start of the text or preceded by
    // whitespace, so typing an inline email address doesn't spuriously pop
    // up the user-search dropdown.
    const userMatch = /(?:^|\s)@([^\s]*)$/.exec(upToCaret)
    if (ticketMatch) {
      setMention({ kind: 'ticket', query: ticketMatch[1], start: caret - ticketMatch[0].length })
      setSelectedIndex(0)
    } else if (userMatch) {
      setMention({ kind: 'user', query: userMatch[1], start: caret - userMatch[1].length - 1 })
      setSelectedIndex(0)
    } else {
      setMention(null)
    }
  }

  function selectMention(item: Ticket | User) {
    if (!mention) return
    const before = value.slice(0, mention.start)
    const after = value.slice(mention.start + 1 + mention.query.length)
    const inserted = mention.kind === 'ticket' ? `#${item.id} ` : `@[${(item as User).display_name}](user:${item.id}) `
    onChange(before + inserted + after)
    setMention(null)

    const caret = before.length + inserted.length
    requestAnimationFrame(() => {
      textareaRef.current?.focus()
      textareaRef.current?.setSelectionRange(caret, caret)
    })
  }

  function handleKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (!mention || !mentionResults || mentionResults.length === 0) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex((i) => (i + 1) % mentionResults.length)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex((i) => (i - 1 + mentionResults.length) % mentionResults.length)
    } else if (e.key === 'Enter' || e.key === 'Tab') {
      e.preventDefault()
      selectMention(mentionResults[selectedIndex])
    } else if (e.key === 'Escape') {
      setMention(null)
    }
  }

  function insertAtCaret(text: string) {
    const el = textareaRef.current
    const caret = el ? el.selectionStart : valueRef.current.length
    const before = valueRef.current.slice(0, caret)
    const after = valueRef.current.slice(caret)
    valueRef.current = before + text + after
    onChange(valueRef.current)

    const newCaret = before.length + text.length
    requestAnimationFrame(() => {
      el?.focus()
      el?.setSelectionRange(newCaret, newCaret)
    })
  }

  function replacePlaceholder(token: string, replacement: string) {
    valueRef.current = valueRef.current.replace(token, replacement)
    onChange(valueRef.current)
  }

  async function uploadImage(file: File) {
    if (!ALLOWED_IMAGE_TYPES.includes(file.type)) {
      setUploadError(t('markdown.unsupportedFileType'))
      return
    }
    setUploadError(null)
    // A unique-per-upload placeholder (rather than a caret offset) so the
    // eventual replace still lands in the right place even if the value
    // changed shape in the meantime.
    const token = `![${t('markdown.uploading')}](pending-${++placeholderCounter})`
    insertAtCaret(token)
    try {
      const { url } = await uploadAttachment(file)
      replacePlaceholder(token, `![image](${url})`)
    } catch (err) {
      replacePlaceholder(token, '')
      setUploadError(getErrorMessage(err, t, 'markdown.uploadFailed'))
    }
  }

  function handlePaste(e: ClipboardEvent<HTMLTextAreaElement>) {
    const items = Array.from(e.clipboardData?.items ?? [])
    const imageItem = items.find((item) => item.kind === 'file' && item.type.startsWith('image/'))
    if (!imageItem) return
    const file = imageItem.getAsFile()
    if (!file) return
    e.preventDefault()
    uploadImage(file)
  }

  function handleDrop(e: DragEvent<HTMLTextAreaElement>) {
    const files = Array.from(e.dataTransfer?.files ?? []).filter((f) => f.type.startsWith('image/'))
    if (files.length === 0) return
    e.preventDefault()
    files.forEach(uploadImage)
  }

  const tabClass = (active: boolean) =>
    `-mb-px border-b-2 px-3 py-1.5 text-sm font-medium ${
      active ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-400 hover:text-gray-600'
    }`

  // Keeps the preview pane roughly the same height as the textarea it
  // replaces, so switching tabs doesn't jump the layout around.
  const minHeightRem = rows * 1.75 + 1

  return (
    <div className={className}>
      <div className="mb-1 flex gap-1 border-b border-gray-200">
        <button type="button" onClick={() => setPreview(false)} className={tabClass(!preview)}>
          {t('markdown.editButton')}
        </button>
        <button
          type="button"
          onClick={() => {
            setMention(null)
            setPreview(true)
          }}
          className={tabClass(preview)}
        >
          {t('markdown.previewButton')}
        </button>
      </div>
      {preview ? (
        <div
          className="overflow-y-auto rounded-b border border-t-0 border-gray-200 bg-gray-50 px-3 py-2"
          style={{ minHeight: `${minHeightRem}rem` }}
        >
          <MarkdownContent text={value || t('markdown.emptyPreview')} />
        </div>
      ) : (
        <div>
          <textarea
            ref={textareaRef}
            value={value}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            onDrop={handleDrop}
            onDragOver={(e) => e.preventDefault()}
            rows={rows}
            placeholder={placeholder}
            className="w-full rounded-b border border-t-0 border-gray-300 px-3 py-2 text-sm"
          />
          {uploadError && <div className="mt-1 text-xs text-red-600">{uploadError}</div>}
          {mention && mentionResults && mentionResults.length > 0 && (
            <ul className="mt-1 max-h-40 overflow-y-auto rounded border border-gray-200 bg-white text-sm shadow-sm">
              {mentionResults.map((item, i) => (
                <li
                  key={item.id}
                  onMouseDown={(e) => {
                    e.preventDefault()
                    selectMention(item)
                  }}
                  className={`cursor-pointer px-2 py-1 ${i === selectedIndex ? 'bg-blue-50' : 'hover:bg-gray-50'}`}
                >
                  {mention.kind === 'ticket' ? (
                    <>
                      <span className="font-mono text-gray-500">#{item.id}</span> {(item as Ticket).title}
                    </>
                  ) : (
                    <>
                      <span className="font-medium">{(item as User).display_name}</span>{' '}
                      <span className="text-gray-400">{(item as User).email}</span>
                    </>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}
