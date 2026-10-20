# Showing a file

**Part of:** [Job](../session.md).

**A session puts a file in front of the person with `show_file`** (`path`), a tool on the agent's door, in place of pasting its text into a message.
Bridge opens it in the side panel to read or edit when he is looking at that job, and otherwise it waits on the job's thread row.
Fleet places a hosted session by its connection, as for `show_window`; a terminal session names its own id.

**`path` is absolute, or relative to the folder the job runs in**, the `cwd` its ledger row holds.
Fleet canonicalises it, and the receipt through the door names the absolute path it kept, so the agent learns what a relative one meant.
A path with nothing at it, a folder, anything that is not a regular file, and a file over 8 MiB (`8 * 1024 * 1024`, the most Bridge's panel reads: `MOST_BYTES` in `apps/desktop/src/main/session-file.ts`) are refused with 422, `fleet.file_missing`, `fleet.file_not_a_file` or `fleet.file_too_large`, naming the path as resolved and what is allowed.
A call that places no job is `fleet.file_unnamed`.
The two caps are written twice and nothing ties them together.

**It is kept as an artifact of form `file`**, `target` the canonical path, with `detail` of `by` `agent`, `title` (the file's name) and `shown`, an RFC 3339 time with milliseconds taken at the call.
`shown` is what makes each call a show Bridge can tell apart: the ledger writes a row only when its detail changed, so a second call for the same path would otherwise publish nothing.
The row is upserted on kind and target and its detail replaced whole.
A file the job wrote is the same row with `by` and `shown` added, and `written` `true` so the row still says the job wrote it.
A picture it looked at becomes form `file`, and writing the file again puts `form` and `title` back and drops the rest.
The ledger names a row with `by` and no `written` "File shown", and any other file row "File written".
A file written through a symlink or a relative path is a second row, because the ledger keeps a written file's path as the agent wrote it and this one is canonical.

Nothing is written to the thread and no window opens. The call is a tool row whose text is `<tool> <path as given>`, which Bridge draws as the file's card.
