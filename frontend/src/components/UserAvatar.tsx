import { useState } from 'react'
import { avatarUrl } from '../api/auth'
import type { User } from '../types/models'

function initials(name: string): string {
  const trimmed = name.trim()
  if (!trimmed) return '?'
  const parts = trimmed.split(/\s+/)
  const chars = parts.length > 1 ? parts[0][0] + parts[1][0] : trimmed.slice(0, 2)
  return chars.toUpperCase()
}

interface UserAvatarProps {
  user: Pick<User, 'id' | 'display_name' | 'avatar_updated_at'>
  size?: number
  className?: string
}

// Tries the uploaded photo first (cache-busted by avatar_updated_at) and
// falls back to an initials circle - both when no avatar was ever set and
// when the image request itself fails for any reason.
export function UserAvatar({ user, size = 32, className }: UserAvatarProps) {
  const [failed, setFailed] = useState(false)
  const src = avatarUrl(user.id, user.avatar_updated_at)

  if (!src || failed) {
    return (
      <div
        style={{ width: size, height: size, fontSize: size * 0.4 }}
        className={`flex flex-shrink-0 items-center justify-center rounded-full bg-blue-100 font-semibold text-blue-700 ${className ?? ''}`}
      >
        {initials(user.display_name)}
      </div>
    )
  }

  return (
    <img
      src={src}
      alt={user.display_name}
      onError={() => setFailed(true)}
      style={{ width: size, height: size }}
      className={`flex-shrink-0 rounded-full object-cover ${className ?? ''}`}
    />
  )
}
