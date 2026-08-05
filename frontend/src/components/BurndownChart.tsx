import { useTranslation } from 'react-i18next'
import type { BurndownPoint } from '../api/milestones'

const WIDTH = 640
const HEIGHT = 320
const PADDING = { top: 16, right: 16, bottom: 32, left: 36 }

function buildPath(values: (number | null)[], x: (i: number) => number, y: (v: number) => number): string {
  let d = ''
  let penDown = false
  values.forEach((v, i) => {
    if (v === null) {
      penDown = false
      return
    }
    const cmd = penDown ? 'L' : 'M'
    d += `${cmd}${x(i).toFixed(1)},${y(v).toFixed(1)} `
    penDown = true
  })
  return d.trim()
}

// A small hand-rolled SVG line chart - the codebase has no general-purpose
// charting library (only frappe-gantt, which is gantt-specific), and two
// lines over N days doesn't warrant adding one.
export function BurndownChart({ points, totalAtStart }: { points: BurndownPoint[]; totalAtStart: number }) {
  const { t } = useTranslation()

  if (points.length === 0) {
    return null
  }

  const maxY = Math.max(totalAtStart, ...points.map((p) => p.remaining), 1)
  const innerWidth = WIDTH - PADDING.left - PADDING.right
  const innerHeight = HEIGHT - PADDING.top - PADDING.bottom

  const x = (i: number) => PADDING.left + (points.length === 1 ? 0 : (i / (points.length - 1)) * innerWidth)
  const y = (v: number) => PADDING.top + innerHeight - (v / maxY) * innerHeight

  const remainingPath = buildPath(points.map((p) => p.remaining), x, y)
  const idealPath = buildPath(points.map((p) => p.ideal), x, y)

  const yTicks = [0, Math.round(maxY / 2), maxY]
  // Show at most ~6 x-axis date labels, evenly spaced, always including the first/last day.
  const labelEvery = Math.max(1, Math.ceil(points.length / 6))
  const xLabels = points
    .map((p, i) => ({ i, date: p.date }))
    .filter(({ i }) => i === 0 || i === points.length - 1 || i % labelEvery === 0)

  return (
    <div className="overflow-x-auto">
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} className="w-full max-w-2xl" role="img">
        {yTicks.map((tick) => (
          <g key={tick}>
            <line
              x1={PADDING.left}
              x2={WIDTH - PADDING.right}
              y1={y(tick)}
              y2={y(tick)}
              stroke="#e5e7eb"
              strokeWidth={1}
            />
            <text x={PADDING.left - 6} y={y(tick)} textAnchor="end" dominantBaseline="middle" fontSize={11} fill="#6b7280">
              {tick}
            </text>
          </g>
        ))}

        {xLabels.map(({ i, date }) => (
          <text key={i} x={x(i)} y={HEIGHT - PADDING.bottom + 16} textAnchor="middle" fontSize={10} fill="#6b7280">
            {date.slice(5)}
          </text>
        ))}

        {idealPath && <path d={idealPath} fill="none" stroke="#9ca3af" strokeWidth={1.5} strokeDasharray="5,4" />}
        <path d={remainingPath} fill="none" stroke="#2563eb" strokeWidth={2} />
      </svg>

      <div className="mt-2 flex items-center gap-4 text-xs text-gray-600">
        <span className="flex items-center gap-1">
          <span className="inline-block h-0.5 w-4 bg-blue-600" />
          {t('burndown.remainingStoryPoints')}
        </span>
        {idealPath && (
          <span className="flex items-center gap-1">
            <span className="inline-block h-0.5 w-4 border-t-2 border-dashed border-gray-400" />
            {t('burndown.ideal')}
          </span>
        )}
      </div>
    </div>
  )
}
