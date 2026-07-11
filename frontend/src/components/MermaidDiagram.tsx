import { useEffect, useId, useState } from 'react'
import mermaid from 'mermaid'

mermaid.initialize({ startOnLoad: false, theme: 'default' })

export function MermaidDiagram({ code }: { code: string }) {
  const rawId = useId().replace(/[^a-zA-Z0-9]/g, '')
  const [svg, setSvg] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    mermaid
      .render(`mermaid-${rawId}`, code)
      .then(({ svg }) => {
        if (!cancelled) {
          setSvg(svg)
          setError(null)
        }
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err))
      })
    return () => {
      cancelled = true
    }
  }, [code, rawId])

  if (error) {
    return <pre className="overflow-x-auto rounded bg-red-50 p-3 text-xs text-red-700">{error}</pre>
  }
  if (!svg) return null

  return <div className="my-2 overflow-x-auto" dangerouslySetInnerHTML={{ __html: svg }} />
}
