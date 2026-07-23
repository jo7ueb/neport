import { visit } from 'unist-util-visit'
import type { Link, Root, Text } from 'mdast'

const USER_MENTION_RE = /@\[([^\]]*)\]\(user:(\d+)\)/g

// Turns "@[Name](user:123)" mention tokens (inserted by MarkdownField's
// mention autocomplete) into "@Name" link nodes with a "user:123" url - not
// a real route (there's no public user-profile page), just a marker the
// MarkdownContent `a` renderer recognizes to style it as a non-navigable
// mention badge instead of a link. Mirrors remarkTicketRefs's structure.
export function remarkUserMentions() {
  return (tree: Root) => {
    visit(tree, 'text', (node: Text, index, parent) => {
      if (!parent || index === undefined || parent.type === 'link') return

      const matches = [...node.value.matchAll(USER_MENTION_RE)]
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
          url: `user:${match[2]}`,
          children: [{ type: 'text', value: `@${match[1]}` }],
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
