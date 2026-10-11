# Where does a standing order get its issues, and does Armada write back? (#91)

**Decided 2026-10-11.**

A picker Drone could read GitHub itself, the way an Epic reads a milestone. The
Cockpit could then only show issues once they had been picked, and never the
backlog the order is choosing from.

**Chosen:** build #91 first. Fleet fetches the tracker, and its two open
questions are answered:

| #91 asked | Answer |
|---|---|
| Does a new issue become work on its own | No. Issues are a view until he dispatches one or an order picks it |
| Does a Job write back to its issue | Yes: a link to the Job and pull request, a pull request that closes the issue, and labels that follow the Job (in progress, set aside) |

**Cost he took:** a tracker adapter comes before the standing order. Fleet needs
write access to issues, and labels it sets can overwrite labels set by hand.

**Where it landed:** #91, which a standing order now waits on.
