# The Helm socket: retired

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**A record of what was.** Helm is gone and the Operator took its place; [The Operator takes Helm's place](the-operator-takes-helms-place.md) has the seam as it is now.

`GET /helm/observe`, `POST /helm/tune` and `GET /helm/file` are gone, with the socket's messages (`opened`, `asked`, `fresh`, `unanswered`, `sent`, `facts`, `missed`, `closed`) and the types that carried them.
Helm is a hosted Job, and its thread is read the way a Job's is: *Helm is a Job*, below.
