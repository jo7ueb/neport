import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { searchTickets } from '../api/tickets'
import type { Ticket } from '../types/models'

interface TicketPickerProps {
  projectId: number
  value: Ticket | null
  onChange: (ticket: Ticket | null) => void
  placeholder?: string
  className?: string
  // A ticket id to leave out of the results, e.g. the ticket currently
  // being edited (so it can't be picked as its own dependency).
  excludeId?: number
}

// Same search-as-you-type dropdown as the "#" mention picker in
// MarkdownField, but as a standalone input for pickers that aren't embedded
// in a textarea (e.g. the dependency form).
export function TicketPicker({ projectId, value, onChange, placeholder, className, excludeId }: TicketPickerProps) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const containerRef = useRef<HTMLDivElement>(null)

  const { data: rawResults } = useQuery({
    queryKey: ['tickets', 'search', query, projectId],
    queryFn: () => searchTickets(query, projectId),
    enabled: open,
    staleTime: 30_000,
  })
  const results = rawResults?.filter((ticket) => ticket.id !== excludeId)

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  function select(ticket: Ticket) {
    onChange(ticket)
    setQuery('')
    setOpen(false)
  }

  function clear() {
    onChange(null)
    setQuery('')
  }

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (!open || !results || results.length === 0) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex((i) => (i + 1) % results.length)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex((i) => (i - 1 + results.length) % results.length)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      select(results[selectedIndex])
    } else if (e.key === 'Escape') {
      setOpen(false)
    }
  }

  if (value) {
    return (
      <div
        className={`flex items-center gap-1 rounded border border-gray-300 bg-gray-50 px-2 py-1 text-sm ${className ?? ''}`}
      >
        <span className="flex-1 truncate">
          #{value.id} {value.title}
        </span>
        <button type="button" onClick={clear} className="text-gray-400 hover:text-red-600">
          ×
        </button>
      </div>
    )
  }

  return (
    <div ref={containerRef} className={`relative ${className ?? ''}`}>
      <input
        type="text"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value)
          setOpen(true)
          setSelectedIndex(0)
        }}
        onFocus={() => setOpen(true)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder ?? t('ticketPicker.placeholder')}
        className="w-full rounded border border-gray-300 px-2 py-1 text-sm"
      />
      {open && results && results.length > 0 && (
        <ul className="absolute z-10 mt-1 max-h-40 w-64 overflow-y-auto rounded border border-gray-200 bg-white text-sm shadow-sm">
          {results.map((ticket, i) => (
            <li
              key={ticket.id}
              onMouseDown={(e) => {
                e.preventDefault()
                select(ticket)
              }}
              className={`cursor-pointer px-2 py-1 ${i === selectedIndex ? 'bg-blue-50' : 'hover:bg-gray-50'}`}
            >
              <span className="font-mono text-gray-500">#{ticket.id}</span> {ticket.title}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
