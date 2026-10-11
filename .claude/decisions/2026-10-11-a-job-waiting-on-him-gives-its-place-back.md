# Does a Job waiting on his answer keep its place?

**Decided 2026-10-11.**

At 1am a Job asks him whether to keep an old setting name or rename it
everywhere. Today the asking Drone keeps its place under the concurrency
bound, so with two places, one unanswered question halves the order overnight
and two stop it. `docs/capabilities/epic-as-one-job.md` filed this as a known
cost rather than fixing it.

**Chosen:** a Job that asks gives its place back. Its branch and worktree stay,
its Drone stands down, and the order takes the next issue. When he answers,
the Job queues for a place and a fresh Drone resumes from the answer and the
branch, the same way an Epic's parent comes back after its children.

**Cost he took:** the asking Drone's own context is lost, and the new one
starts from the branch and the question.

**Where it landed:** not built. It applies to every Job, not only those a
standing order picked.
