---
kind: tool
name: Risks
---
What could go wrong that the change does not handle: shared state, ordering, a caller that was not updated, a failure path nobody reads, a limit the new code can exceed. Read the callers and the neighbouring code in the checkout, not only the diff. Report a risk only when you can name the place it would bite and what would happen there.
