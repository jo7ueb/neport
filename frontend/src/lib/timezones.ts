// Used only as a fallback if the runtime doesn't support
// Intl.supportedValuesOf (Chrome 99 / Firefox 93 / Safari 15.4, all from
// 2021-2022) - see ALL_TIMEZONES below, which is what the dropdowns
// actually render.
const FALLBACK_TIMEZONES = [
  'Asia/Tokyo',
  'UTC',
  'Asia/Shanghai',
  'Asia/Singapore',
  'Asia/Kolkata',
  'Europe/London',
  'Europe/Paris',
  'America/New_York',
  'America/Los_Angeles',
  'Australia/Sydney',
]

function utcOffsetMinutes(tz: string, at: Date): number {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tz,
    hourCycle: 'h23',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
    .formatToParts(at)
    .reduce<Record<string, string>>((acc, p) => {
      acc[p.type] = p.value
      return acc
    }, {})
  const asUtc = Date.UTC(
    Number(parts.year),
    Number(parts.month) - 1,
    Number(parts.day),
    Number(parts.hour),
    Number(parts.minute),
    Number(parts.second)
  )
  return Math.round((asUtc - at.getTime()) / 60000)
}

// The offset is what actually matters to a reader ("UTC+9h"); the IANA name
// is only useful as a reference for DST-observing zones, so it's kept as a
// secondary label rather than the primary display. Computed from the raw
// offset (rather than Intl's `timeZoneName: 'shortOffset'`) since that
// formatting option isn't reliably supported everywhere.
export function utcOffsetLabel(tz: string, at: Date = new Date()): string {
  let total: number
  try {
    total = utcOffsetMinutes(tz, at)
  } catch {
    return tz
  }
  if (total === 0) return 'UTC'
  const sign = total < 0 ? '-' : '+'
  const abs = Math.abs(total)
  const hours = Math.floor(abs / 60)
  const minutes = abs % 60
  return minutes === 0 ? `UTC${sign}${hours}h` : `UTC${sign}${hours}:${String(minutes).padStart(2, '0')}`
}

export function timezoneOptionLabel(tz: string): string {
  return `${utcOffsetLabel(tz)} (${tz})`
}

// The backend accepts any valid IANA name (validated via chrono-tz), so the
// dropdown should offer the full canonical set rather than a small curated
// shortlist - Intl.supportedValuesOf('timeZone') already returns every
// IANA zone with no duplicates. Sorted by current UTC offset (then name)
// since that's what a user is actually scanning for.
function computeAllTimezones(): string[] {
  try {
    return Intl.supportedValuesOf('timeZone')
  } catch {
    return FALLBACK_TIMEZONES
  }
}

export const ALL_TIMEZONES = ((): string[] => {
  const now = new Date()
  return computeAllTimezones()
    .slice()
    .sort((a, b) => {
      const diff = utcOffsetMinutes(a, now) - utcOffsetMinutes(b, now)
      return diff !== 0 ? diff : a.localeCompare(b)
    })
})()
