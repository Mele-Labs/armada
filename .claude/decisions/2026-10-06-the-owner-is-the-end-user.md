# Who is Armada's owner when a session works on it?

**Decided 2026-10-06.**

The owner is an end user of Armada. He says how he wants to use the system, and sessions make it happen. He does not run the repository's release process by hand, and a session does not stop partway to ask whether to carry on.

**Chosen: every change follows one path, with no question between its steps.**

```
propose --> implement --> walk it, if visual --> preview and adopt --> land
                              his OK is the go-ahead
```

- **A visual change is walked, and his OK on the walk is the go-ahead.** Nothing he would see lands before it.
- **Preview and adopt come next.** `scripts/preview` merges the in-flight branches. When the change reaches Fleet or Bridge, `scripts/preview --restart --adopt` moves his own Fleet and Bridge onto it, so he uses what he asked for.
- **Landing follows without a question.** A session never asks "land it?" between the walk and the land. Pull request #1807 put this into the `work-issue` skill, and #1808 changed the land step to a pull request.

**Not changed:** a decision that reverses one he made earlier, or that changes what a person can do, is still his and still asked first (`asking-a-person`). The path removes the question about landing, not about what to build.

**Where it landed:** `work-issue`, through #1807.
