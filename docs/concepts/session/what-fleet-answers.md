# What Fleet answers

**Part of:** [Job](../session.md).

| Operation | Answers |
|---|---|
| `list_sessions` | Every job, most recently seen first, narrowed by repository, `live` or `ended`, and `q`: a title, a branch, a pull request number or a slot |
| `who_owns` | Every holder of a branch, a pull request or a slot, the ones still holding it first. Empty is an answer |
| `session.changed` | One job whole, whenever a fact changed something |

Both reads are on the agent's door, so a job can ask who else holds the branch it is about to take.
