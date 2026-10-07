import type { Register } from 'claude-code'

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const id = await $.session.id()
    await $.fs.write(
      '/path/to/probe/marker-' + id + '.txt',
      'started cwd=' + e.cwd + ' interactive=' + e.isInteractive + '\n',
    )
    return next(e)
  })
}
