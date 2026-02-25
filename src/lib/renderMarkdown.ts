import { marked, type Renderer } from 'marked'
import DOMPurify from 'dompurify'

// Regex matches `[text](:/32hexchars)` but NOT `![text](:/...)` (image syntax).
// The negative lookbehind `(?<!!)` ensures we skip image links.
const JOPLIN_LINK_RE = /(?<!!)(\[[^\]]*\])\(:\/([0-9a-f]{32})\)/g

/**
 * Transform Joplin internal note links into a navigable `joplin-note://` scheme
 * before handing the markdown off to `marked`. This avoids teaching marked about
 * Joplin's `:/<id>` URL format.
 *
 * Input:  [My Note](:/abcdef1234567890abcdef1234567890)
 * Output: [My Note](joplin-note://abcdef1234567890abcdef1234567890)
 */
function transformJoplinLinks(markdown: string): string {
  return markdown.replace(JOPLIN_LINK_RE, '$1(joplin-note://$2)')
}

// ── Custom link renderer ────────────────────────────────────────────────────

const linkClass = 'text-indigo-400 hover:underline cursor-pointer'

const renderer: Partial<Renderer> = {
  link({ href, title, text }: { href: string; title: string | null | undefined; text: string }) {
    const titleAttr = title ? ` title="${title}"` : ''

    if (href.startsWith('joplin-note://')) {
      // Internal note link — no target, handled via event delegation in DetailPanel
      return `<a href="${href}" class="${linkClass}"${titleAttr}>${text}</a>`
    }

    // External link — open in system browser
    return `<a href="${href}" class="${linkClass}" target="_blank" rel="noopener noreferrer"${titleAttr}>${text}</a>`
  },
}

marked.use({ renderer })

// ── DOMPurify configuration ─────────────────────────────────────────────────

const ALLOWED_TAGS = [
  'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
  'p', 'br', 'hr',
  'strong', 'em', 'del',
  'code', 'pre',
  'blockquote',
  'ul', 'ol', 'li',
  'table', 'thead', 'tbody', 'tr', 'th', 'td',
  'a', 'span', 'div',
]

const ALLOWED_ATTR = ['href', 'class', 'target', 'rel', 'title']

const PURIFY_CONFIG: Parameters<typeof DOMPurify.sanitize>[1] = {
  ALLOWED_TAGS,
  ALLOWED_ATTR,
}

// Allowlist the URI schemes permitted in `href` attributes.
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') {
    const href = node.getAttribute('href') ?? ''
    const allowed =
      href.startsWith('joplin-note://') ||
      href.startsWith('http://') ||
      href.startsWith('https://')
    if (!allowed) {
      node.removeAttribute('href')
    }
  }
})

// TODO: Phase 2 — image support
// Add 'img' to ALLOWED_TAGS, add 'src'/'alt' to ALLOWED_ATTR, and add a
// Joplin resource URL transformer here that converts `:/<id>` image src values
// to the appropriate app-served resource URL (e.g. via a Tauri command).

// ── Public API ──────────────────────────────────────────────────────────────

/**
 * Render a Joplin note body (Markdown) to a sanitized HTML string safe for
 * `dangerouslySetInnerHTML`. Never returns `undefined` — returns `''` on error.
 *
 * @param markdown - Raw markdown text from the Joplin database
 * @returns Sanitized HTML string ready for injection into the DOM
 */
export function renderMarkdown(markdown: string): string {
  const transformed = transformJoplinLinks(markdown)
  // marked.parse() is synchronous when no async extensions are registered.
  const raw = marked.parse(transformed, { async: false }) as string
  return DOMPurify.sanitize(raw, PURIFY_CONFIG)
}
