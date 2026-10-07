import type { Register } from 'claude-code'

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const id = await $.session.id()
    await $.fs.write(
      '/private/tmp/claude-501/-Users-nickmele-Development-armada/d42cb58b-6440-4cec-8ce4-49863613ae18/scratchpad/probe/marker-' + id + '.txt',
      'started cwd=' + e.cwd + ' interactive=' + e.isInteractive + '\n',
    )
    return next(e)
  })
}
