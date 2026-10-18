// A whole-file read of a large file stays in the context for every turn after it. The guard
// turns the first one away with the file's outline, so the agent reads the range it needs.
// Pure, so each reading is a test and not a session.

/** Past this many bytes (about 10k tokens), a Read with no range is turned away once. */
export const WHOLE_READ_MOST = 40 * 1024

/** The engine reads these as pages or images, not lines: the guard leaves them alone. */
const NOT_LINES = /\.(pdf|ipynb|png|jpe?g|gif|webp|bmp|ico|icns)$/i

const OUTLINE_MOST = 60
const LINE_MOST = 100

/** What a Read asked for, as far as the guard reads it. */
export type ReadAsked = { file_path: string; offset?: number; limit?: number; pages?: string; allow_large?: boolean }

/** Whether the guard looks at this Read at all: a whole read of a file read as lines. */
export function wholeRead(e: ReadAsked): boolean {
  return e.offset === undefined && e.limit === undefined && e.pages === undefined && e.allow_large !== true && !NOT_LINES.test(e.file_path)
}

/**
 * The lines that say where things are: a markdown heading, or a top-level declaration in
 * Rust, TypeScript or Python. Numbered from 1, as Read's `offset` counts.
 */
export function outlineOf(path: string, text: string): string[] {
  const markdown = /\.mdx?$/i.test(path)
  const marks = markdown
    ? /^#{1,4} /
    : /^(pub(\([a-z]+\))? )?(async )?(fn|struct|enum|trait|impl|mod|type|const|static|class|def|function|interface|export) /
  const found: string[] = []
  let fenced = false
  text.split('\n').forEach((line, at) => {
    if (markdown && line.startsWith('```')) fenced = !fenced
    if (!fenced && marks.test(line)) found.push(`${at + 1}: ${line.length > LINE_MOST ? `${line.slice(0, LINE_MOST)}…` : line}`)
  })
  return found
}

/** What the agent is told in place of the file. */
export function turnedAway(path: string, size: number, text: string): string {
  const lines = text.split('\n').length
  const outline = outlineOf(path, text)
  const shown = outline.slice(0, OUTLINE_MOST)
  const more = outline.length > shown.length ? `\n… ${outline.length - shown.length} more; Grep the file for the rest.` : ''
  const where = shown.length > 0 ? `\n\nWhere things are (line: text):\n${shown.join('\n')}${more}` : ''
  return (
    `Not read: ${path} is ${Math.round(size / 1024)} KB, ${lines} lines, about ${Math.round(size / 4000)}k tokens, ` +
    `and a whole read stays in the context for every turn after this one. ` +
    `Grep it, or Read again with offset and limit for the part you need. ` +
    `Reading it whole again goes through.${where}`
  )
}
