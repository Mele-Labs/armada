// What a Read, Grep or Glob is about, for handing a Job the page of the feature that owns it
// (`docs/concepts/feature-catalog.md`). Fleet maps a path to features and decides whether the page was
// handed before; this only says which file or folder a call reaches. Pure, so each reading is a test.

/** What the three tools take that names a place. */
export type SearchAsked = { file_path?: string; path?: string; glob?: string; pattern?: string }

/** The folder in front of a glob's first wildcard, or the whole pattern where it has none. */
export function staticPrefix(pattern: string): string {
  const kept: string[] = []
  for (const segment of pattern.split('/')) {
    if (/[*?[\]{}]/.test(segment)) break
    kept.push(segment)
  }
  return kept.join('/')
}

/** `place` below `root`, as a repository relative path, or undefined where it is the root itself or outside it. */
function under(root: string, place: string): string | undefined {
  const base = root.replace(/\/+$/, '')
  let relative = place
  if (place.startsWith('/')) {
    if (place !== base && !place.startsWith(`${base}/`)) return undefined
    relative = place.slice(base.length)
  }
  relative = relative.replace(/^(\.\/)+/, '').replace(/^\/+/, '').replace(/\/+$/, '')
  return relative === '' || relative === '.' || relative.startsWith('../') ? undefined : relative
}

/**
 * The repository relative place a call reaches, one at most: a Read's file, a Grep's or Glob's folder joined to the
 * folder in front of its glob. A search of the whole repository reaches none, and neither does one outside `root`.
 */
export function placeOfCall(tool: string, e: SearchAsked, root: string): string | undefined {
  if (tool === 'Read') return typeof e.file_path === 'string' ? under(root, e.file_path) : undefined
  if (tool !== 'Grep' && tool !== 'Glob') return undefined
  const pattern = tool === 'Grep' ? e.glob : e.pattern
  const folder = typeof e.path === 'string' && e.path !== '' ? e.path : undefined
  const prefix = typeof pattern === 'string' ? staticPrefix(pattern) : ''
  // An absolute pattern stands alone; a relative one is read from the folder.
  const joined = prefix.startsWith('/') || folder === undefined ? prefix : prefix === '' ? folder : `${folder.replace(/\/+$/, '')}/${prefix}`
  return joined === '' ? undefined : under(root, joined)
}

/**
 * What a shell line does to the commit it makes, or undefined where it makes none. `all` says the same line stages more
 * than is staged now: a `git add` before the commit, or `-a` on it.
 */
export function commitStaging(command: string): { all: boolean } | undefined {
  if (!/\bgit\s[^|;&\n]*\bcommit\b/.test(command)) return undefined
  const all = /\bgit\s+add\b/.test(command) || /\bcommit\b[^|;&\n]*\s(--all|-[a-zA-Z]*a[a-zA-Z]*)(\s|$)/.test(command)
  return { all }
}
