// What a session's own words and tool results say about what it holds. Pure, so
// each reading is a test and not a session.

const TITLE_MOST = 80

/** The first line of a prompt as a title, or nothing for a command or a blank. */
export function titleOf(prompt: string): string | undefined {
  const line = prompt
    .split('\n')
    .map(one => one.trim())
    .find(one => one !== '')
  if (line === undefined || line.startsWith('/')) return undefined
  const flat = line.replace(/\s+/g, ' ')
  return flat.length > TITLE_MOST ? `${flat.slice(0, TITLE_MOST - 1)}…` : flat
}

/** Whether a Bash command may have moved the branch or the directory. */
export function mayMoveBranch(command: string): boolean {
  return /\bgit\b[^|;&]*\b(checkout|switch|branch|commit|merge|rebase|pull|worktree|reset|cherry-pick|clone)\b/.test(
    command,
  )
}

export type PullRequest = { number: string; url: string }

/** Pull requests a result names by their address. */
export function pullRequestsIn(text: string): PullRequest[] {
  const found = new Map<string, PullRequest>()
  for (const hit of text.matchAll(/https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/pull\/(\d+)/g)) {
    found.set(hit[1], { number: hit[1], url: hit[0] })
  }
  return [...found.values()]
}

export type GhAct = { act: 'create' | 'merge' | 'close'; number?: string }

/** A `gh pr` command that makes, lands or closes a pull request. */
export function ghAct(command: string): GhAct | undefined {
  const hit = /\bgh\s+pr\s+(create|merge|close)\b([^|;&]*)/.exec(command)
  if (hit === null) return undefined
  const number = /(?:^|\s)#?(\d+)(?:\s|$)/.exec(hit[2])?.[1]
  return { act: hit[1] as GhAct['act'], number }
}

const DISPATCH = /^mcp__armada-fleet__(propose_job|propose_from_request|approve_dispatch|redispatch_job)$/

export function isDispatch(tool: string): boolean {
  return DISPATCH.test(tool)
}

/** Job ids in a dispatch's answer, which are ULIDs. */
export function jobIdsIn(text: string): string[] {
  return [...new Set(text.match(/\b[0-9A-HJKMNP-TV-Z]{26}\b/g) ?? [])].slice(0, 10)
}

/** Whom a delivery came from, as far as the delivery says: never a session id. */
export function senderOf(origin: { kind: string; teammate?: string }): string {
  return origin.teammate ?? origin.kind
}

export function micros(usd: number | undefined): number | undefined {
  return usd === undefined ? undefined : Math.round(usd * 1_000_000)
}
