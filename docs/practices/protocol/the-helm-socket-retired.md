# The Helm socket: retired

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`GET /helm/observe`, `POST /helm/tune` and `GET /helm/file` are gone, with the socket's messages (`opened`, `asked`, `fresh`, `unanswered`, `sent`, `facts`, `missed`, `closed`) and the types that carried them.
Helm is a hosted Job, and its thread is read the way a Job's is: *Helm is a Job*, below.
