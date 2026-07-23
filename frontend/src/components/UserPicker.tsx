import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { searchUsers } from '../api/users'
import type { User } from '../types/models'

interface UserPickerProps {
  value: User | null
  onChange: (user: User | null) => void
  placeholder?: string
  className?: string
  // User ids to leave out of the results, e.g. users already picked in
  // other rows of the same form (so the same person can't be added twice).
  excludeIds?: number[]
  // Restrict candidates to this list, filtered client-side as the user
  // types, instead of searching every user in the system via the API - e.g.
  // a ticket's assignee should only be pickable from the project's members.
  candidates?: User[]
}

// Same search-as-you-type dropdown as TicketPicker, for forms where a plain
// <select> with every user in the system is unusable (see ticketPicker.tsx
// for the shared UX rationale).
export function UserPicker({ value, onChange, placeholder, className, excludeIds, candidates }: UserPickerProps) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const containerRef = useRef<HTMLDivElement>(null)

  const { data: rawResults } = useQuery({
    queryKey: ['users', 'search', query],
    queryFn: () => searchUsers(query),
    enabled: open && candidates === undefined,
    staleTime: 30_000,
  })
  const matched = candidates
    ? candidates.filter(
        (u) =>
          u.display_name.toLowerCase().includes(query.toLowerCase()) ||
          u.email.toLowerCase().includes(query.toLowerCase())
      )
    : rawResults
  const results = matched?.filter((u) => !excludeIds?.includes(u.id))

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  function select(user: User) {
    onChange(user)
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
        className={`flex items-center gap-1 rounded border border-gray-300 bg-gray-50 px-2 py-1.5 text-sm ${className ?? ''}`}
      >
        <span className="flex-1 truncate">
          {value.display_name} ({value.email})
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
        placeholder={placeholder ?? t('userPicker.placeholder')}
        className="w-full rounded border border-gray-300 px-2 py-1.5 text-sm"
      />
      {open && results && results.length > 0 && (
        <ul className="absolute z-10 mt-1 max-h-48 w-full min-w-64 overflow-y-auto rounded border border-gray-200 bg-white text-sm shadow-sm">
          {results.map((u, i) => (
            <li
              key={u.id}
              onMouseDown={(e) => {
                e.preventDefault()
                select(u)
              }}
              className={`cursor-pointer px-2 py-1 ${i === selectedIndex ? 'bg-blue-50' : 'hover:bg-gray-50'}`}
            >
              {u.display_name} <span className="text-gray-400">({u.email})</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
