import { visit } from 'unist-util-visit'
import type { Link, Root, Text } from 'mdast'

const TICKET_REF_RE = /#(\d+)/g

// Turns bare "#123" mentions in Markdown text into links to /tickets/123
// (resolved to the right project by TicketRedirectPage). Only rewrites plain
// text nodes - runs after remark-gfm/remark-math so autolinked URLs and math
// spans are already their own node types and never get split up here, and
// skips text that's already inside a link to avoid nesting one inside
// another.
export function remarkTicketRefs() {
  return (tree: Root) => {
    visit(tree, 'text', (node: Text, index, parent) => {
      if (!parent || index === undefined || parent.type === 'link') return

      const matches = [...node.value.matchAll(TICKET_REF_RE)]
      if (matches.length === 0) return

      const replacement: (Text | Link)[] = []
      let lastEnd = 0
      for (const match of matches) {
        const start = match.index ?? 0
        if (start > lastEnd) {
          replacement.push({ type: 'text', value: node.value.slice(lastEnd, start) })
        }
        replacement.push({
          type: 'link',
          url: `/tickets/${match[1]}`,
          children: [{ type: 'text', value: match[0] }],
        })
        lastEnd = start + match[0].length
      }
      if (lastEnd < node.value.length) {
        replacement.push({ type: 'text', value: node.value.slice(lastEnd) })
      }

      parent.children.splice(index, 1, ...replacement)
      return index + replacement.length
    })
  }
}
