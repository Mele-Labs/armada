import type { Register } from 'claude-code'

const allowed = new Set<string>()
const keyOf = (input: Record<string, unknown>) => String(input.command ?? '')

const lines: string[] = []

async function note($: any, line: string): Promise<void> {
  lines.push(`${await $.clock.now()} ${line}`)
  await $.fs.write('/tmp/perm-spike/log.txt', lines.join('\n'))
}

export const register: Register = on => {
  on('classic.PermissionRequest', async ($, e, next) => {
    const out = await next(e)
    void note($, `permission_request ${e.tool_name}`)
    return out
  })
  on('tool.check', async ($, e, next) => {
    if (allowed.has(keyOf((e.input ?? {}) as Record<string, unknown>))) {
      void note($, 'check allowed by bridge')
      return { decision: 'allow' }
    }
    return next(e)
  })
  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    const key = keyOf(e as Record<string, unknown>)
    const terminal = next(e).then(ran => ({ ran }))
    const bridge = $.clock.sleep(8000).then(() => ({ bridge: true as const }))
    const first = await Promise.race([terminal, bridge])
    if ('ran' in first) {
      void note($, 'terminal answered first')
      return first.ran
    }
    void note($, 'bridge allows: running again')
    allowed.add(key)
    try {
      const ran = await next(e)
      void note($, `second run: isError=${ran.isError} text=${String(ran.text ?? '').slice(0, 60)}`)
      return ran
    } finally {
      allowed.delete(key)
    }
  })
}
