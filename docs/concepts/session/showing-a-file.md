# Showing a file

**Part of:** [Job](../session.md).

**A session puts a file in front of the person with `show_file`** (`path`), a tool on the agent's door, in place of pasting its text into a message.
Bridge opens a file in the side panel to read or edit when he is looking at that job, and otherwise it waits on the job's thread row.
A picture or a video opens no panel by itself: [it draws in the thread](#a-picture-or-a-video).
Fleet places a hosted session by its connection, as for `show_window`; a terminal session names its own id.

**`path` is absolute, or relative to the folder the job runs in**, the `cwd` its ledger row holds.
Fleet canonicalises it, and the receipt through the door names the absolute path it kept, so the agent learns what a relative one meant.
A path with nothing at it, a folder, anything that is not a regular file, a file over 8 MiB (`8 * 1024 * 1024`, the most Bridge's panel reads: `MOST_BYTES` in `apps/desktop/src/main/session-file.ts`), and a video over 256 MiB (`256 * 1024 * 1024`) are refused with 422, `fleet.file_missing`, `fleet.file_not_a_file` or `fleet.file_too_large`, naming the path as resolved and what is allowed.
A call that places no job is `fleet.file_unnamed`.
The 8 MiB cap is written twice, in Fleet and in Bridge, and nothing ties the two together.
The video cap is Fleet's alone: nothing reads a video whole.

**It is kept as an artifact of form `file`**, `target` the canonical path, with `detail` of `by` `agent`, `title` (the file's name) and `shown`, an RFC 3339 time with milliseconds taken at the call.
`shown` is what makes each call a show Bridge can tell apart: the ledger writes a row only when its detail changed, so a second call for the same path would otherwise publish nothing.
The row is upserted on kind and target and its detail replaced whole; each replacement is also a version of it, which `get_attachment_history` reads back ([What is kept](what-is-kept.md), *Every version*).
A file the job wrote is the same row with `by` and `shown` added, and `written` `true` so the row still says the job wrote it.
A picture it looked at becomes form `file`, and writing the file again puts `form` and `title` back and drops the rest.
The ledger names a row with `by` and no `written` "File shown", and any other file row "File written".
A file written through a symlink or a relative path is a second row, because the ledger keeps a written file's path as the agent wrote it and this one is canonical.

A person reading the file notes lines of it back to the job: [Noting what you read](noting-what-you-read.md).

Nothing is written to the thread and no window opens.
The call is a tool row whose text is `<tool> <path as given>`, which Bridge draws as the file's card, with the picture or the video under it where it is one.

## A picture or a video

**It draws in the Job's thread, under the file's card and sized to the thread, on the Mac and on the phone.**
A picture is a file ending `png`, `jpg`, `jpeg`, `gif`, `webp`, `heic` or `svg`, and a video one ending `mp4`, `mov` or `webm`, in any case.
Neither opens the side panel by itself; the card's "Opened" line opens a picture in the panel, with [notes](noting-what-you-read.md), and a video in the Mac's own player.

**A press on a picture opens it full screen.**

| Where | Zoom, move and close |
|---|---|
| The Mac | The wheel, a double click, a drag, `+`, `-`, `0`, the arrow keys, and `Esc` |
| The phone | Pinch, drag, double tap |

**A video that cannot play says so in a line with an Open video button.**
The button opens the file in the Mac's own player, or in the phone's browser.

**Bridge plays a video from disk in byte ranges, through its own scheme, and only for a path the Job's ledger names.**
Fleet never reads a video.
The phone gets it through the Phone Gateway: [Pocket](../pocket.md), *What keeps it closed*.
