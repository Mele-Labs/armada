# A quick terminal

**Part of:** [Job](../session.md).

A quick terminal is a Job the person starts in a second to run his own commands.
It opens straight onto its terminal `t1`, and its agent waits in the thread and does nothing until he writes.
Fleet starts one with `start_terminal_job` (`POST /sessions/terminal/start`), and the native Bridge sends it from the dispatch sheet, a key and the command palette.

**Where it runs.**

| He picks | The Job | Its terminals |
|---|---|---|
| A project, by its Manifest or its implicit id | That repository's Job | Its checkout, then its slot once it leases one |
| A folder inside a served repository | That repository's Job, the deepest where one holds another | That folder, in the checkout and then in the slot |
| Any other folder | A loose folder: no Manifest, never a slot | That folder |

**A loose folder is left as it is.**
Nothing makes it a repository and nothing serves it, so it is not added to his repositories.
The agent writes in the folder in place, as he does in his own shell.

**The agent costs nothing while it waits.**
The Job has no workflow, by choice, no start card and no proposer reading, and no agent process starts until his first message, as for any Job Fleet hosts.
In a repository it leases a slot at its first write, as any Job does.

**Each message he sends carries the terminals.**
Ahead of his words the agent reads what each open terminal shows, its last 60 lines, so it answers from the terminal without having to ask for it.
It types into one with `type_in_terminal` only when he asks it to.
A note or another Job's message does not carry them.

**A title he does not type names the folder**, as *Terminal in shop*: the folder he picked, or the repository's for a project.

**While it waits it is live and idle**, never ended or done: its agent has not run, and nothing has finished.
Its row is marked `blank`, not started, though Fleet gave it a title.

**Lists mark it** with `quick_terminal`, so Bridge opens it onto `t1` and draws it apart from a working Job.
A Job that replaces it on a handoff keeps the mark, and its terminal folder.
