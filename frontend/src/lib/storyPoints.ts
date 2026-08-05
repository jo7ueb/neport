export const STORY_POINT_OPTIONS = [1, 2, 3, 5, 8, 13, 21] as const

export function formatEstimatedHours(hours: number): { hours: number; minutes: number } {
  const totalMinutes = Math.round(hours * 60)
  return {
    hours: Math.floor(totalMinutes / 60),
    minutes: totalMinutes % 60,
  }
}
