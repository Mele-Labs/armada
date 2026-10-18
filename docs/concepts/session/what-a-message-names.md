# What a message names

**Part of:** [Job](../session.md).

A message to a hosted Job can name things with `@` and `#`, sent as `mentions` and kept on its `message` row as `tags`.
**Fleet writes the line that tells the agent** (`The person named: ...`), ahead of the words, and the thread keeps the words as typed.

| Tag `kind` | `id` | The agent is told |
|---|---|---|
| `session` | The Job's id | Its title, and its address to write to with `SendMessage` where Fleet hosts it |
| `pull_request` | The number | `pull request #219 "title"`, and its address where the tag has one |
| `issue` | The number | `issue #214 "title"` and its address. A tag's title begins with `#` and the number, and the line drops that so the number is said once |
| `branch` | The name | The branch |

**`@` offers the other Jobs, and once letters follow it the repository's files that match them.** A file is written into the words as `@path` and is not a tag: the agent reads the path itself.
Pull requests are `#`'s, and no trigger offers a branch, though the kind is still accepted.

**`#` at the start of a word offers the repository's open issues and open pull requests**, matched on the number, or on the key where the tracker names issues by one (`#ops` offers `OPS-7` and `OPS-12`, whatever the case), and reads one number that is neither (a merged pull request, a closed issue) when it is typed whole.
The repository is the Job's own.
Helm's box offers the same, for the repository its conversation answers for (`helm.md`, *The message box*).
`url` on a tag is optional and only the two forge kinds carry it.
