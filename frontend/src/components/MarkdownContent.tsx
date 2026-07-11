import { isValidElement } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import rehypeKatex from 'rehype-katex'
import { Link } from 'react-router-dom'
import { remarkTicketRefs } from '../lib/remarkTicketRefs'
import { useSettings } from '../context/SettingsContext'
import { MermaidDiagram } from './MermaidDiagram'

function isMermaidCodeElement(node: unknown): node is { props: { className?: string; children?: unknown } } {
  return isValidElement(node) && typeof (node.props as { className?: string })?.className === 'string'
    && /language-mermaid/.test((node.props as { className: string }).className)
}

export function MarkdownContent({ text }: { text: string }) {
  const { katexEnabled } = useSettings()
  const remarkPlugins = katexEnabled
    ? [remarkGfm, remarkMath, remarkTicketRefs]
    : [remarkGfm, remarkTicketRefs]
  const rehypePlugins = katexEnabled ? [rehypeKatex] : []

  return (
    <div className="prose prose-sm max-w-none">
      <ReactMarkdown
        remarkPlugins={remarkPlugins}
        rehypePlugins={rehypePlugins}
        components={{
          a: ({ href, children }) =>
            href?.startsWith('/') ? (
              <Link to={href}>{children}</Link>
            ) : (
              <a href={href} target="_blank" rel="noopener noreferrer">
                {children}
              </a>
            ),
          // A ```mermaid fenced block renders as <pre><code class="language-mermaid">
          // by default - unwrap the <pre> so the diagram isn't shown inside a
          // monospace code box.
          pre: ({ children }) => {
            const child = Array.isArray(children) ? children[0] : children
            if (isMermaidCodeElement(child)) return <>{child}</>
            return <pre>{children}</pre>
          },
          code: ({ className, children, ...props }) => {
            if (className && /language-mermaid/.test(className)) {
              return <MermaidDiagram code={String(children).replace(/\n$/, '')} />
            }
            return (
              <code className={className} {...props}>
                {children}
              </code>
            )
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  )
}
