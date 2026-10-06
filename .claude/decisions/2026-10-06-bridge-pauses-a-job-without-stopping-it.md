# How does a person pause a Job from Bridge?

**Decided 2026-10-05, built 2026-10-06.**

He said: "I should be able to pause a job and not stop it. Stopping it is terminal. I want to just pause jobs sometimes to kill all of its process and resume it later." And: "I should be able to tell a job to release the worktree for now."

**Chosen: Pause and Resume as acts on the Job, with the pause shown beside its status.**

- **A Job paused at a review gate reads Needs review with a paused mark.** The mark sits beside the real badge and never replaces it. A running Job that is paused reads queued with the reason paused.
- **An act on a paused Job opens a confirm that offers Resume.** Approve, request changes, restart and override come back `fleet.paused`. Resume only resumes; the act is not sent, and the person presses it again.
- **A full pool is not the confirm's to explain.** The Job reads waiting for a slot afterwards.
- **The mark is an icon with a tooltip**, naming who paused it (`person` or `fleet`), when, and the branch. `circle-pause` marks a pause and `play` is Resume. `pause` is Freeze dispatch's and is not reused.
- **Pause is offered on the Board row, in Job detail and on a Cleanup tile**, Resume beside it. Pause asks first and lists git effects; Resume is sent at once.

**Not built:** Pause on a `queued` row of the Board, because a row carries no slot. Cleanup offers it, where the pool is read.

**Where it landed:** `bridge/pause-and-resume`, slice 3. The wire is `park_job`, `resume_job`, `job.paused` and `job.resumed`, protocol 23.39.
