# Spike 28 — Can a session be forked from a dead one?

**Yes, and under an id Fleet chooses.** `--resume <old> --fork-session
--session-id <new>` starts a process whose conversation is a copy of the old
one's, written under `<new>`. The old id is found wherever its transcript was
written, so a session started in another directory forks without being moved.

Measured on 7 Oct 2026 against Claude Code 2.1.292, `--model haiku`,
`--setting-sources project,local`, in scratch directories. Nothing reported into
the Fleet, so no `ended` fact was owed.

| Step | What happened |
|---|---|
| `claude --help` | `--fork-session`: "When resuming, create a new session ID instead of reusing the original (use with --resume or --continue)" |
| Session A in directory `a`, `--session-id <A>`, told to remember a word | answered, and wrote `<A>.jsonl` under `a`'s project directory |
| `--resume <A> --fork-session` run in directory `b`, asked for the word | answered with the word. A fresh id, the file under `b`'s project directory |
| The same, run in `a` | the same answer |
| `--resume <A> --fork-session --session-id <N>` in `b` | accepted. The result's `session_id` is `<N>`, the file is `<N>.jsonl` under `b`'s directory |
| `--resume <N>` alone, afterwards | answered with the word, so the fork resumes as any session does |

## Things Fleet depends on

- **Fleet mints the fork's id.** It passes `--session-id` with `--fork-session`,
  so the ledger's row and the CLI's are one from the first process, as for a
  session that is not a fork. Without it the CLI picks an id Fleet would have
  to read back from the stream.
- **Only the first process forks.** Once it has run, the session resumes by its
  own id, so `--fork-session` is not passed again. A second fork of the same
  old session is a second new session.
- **The directory the old session ran in does not matter.** A terminal session's
  transcript lives under the project directory of the directory it started in.
  The fork run in `b` found a session written under `a`, so the lookup is by id
  across project directories. It was measured with sessions started by `-p`, not
  an interactive one; the transcript file is the same shape and the lookup is by id.
- **The old transcript is read, not changed.** The fork writes its own file.
- **A fork does not carry the old session's slot or branch.** The conversation
  mentions them; the files are the repository's. The fork starts in the main
  checkout and takes its own slot at its first write.
