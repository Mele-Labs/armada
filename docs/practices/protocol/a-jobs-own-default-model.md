# A Job's own default model

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**One required field and one setting.** `ModelChoices.session` is `models.session` as it stands when `list_models` is asked, always a member of `models`, and `default` stays `models.default`. It is required, not additive: Bridge and Fleet ship together, and a Fleet that omits it fails Bridge's read rather than drawing a picker with no Job default. Bridge's half is `packages/protocol/src/setup.ts`, written by hand.

| Setting | Ships as | Read by |
| --- | --- | --- |
| `models.session` | The roster's strongest model, derived in `adapters` | A hosted Job whose process starts with no model chosen, and the step down |
| `models.default` | The Drone's model | A Job proposed with no model, and nothing that starts a Session |

**No environment variable** names `models.session`; `ARMADA_MODEL` still names `models.default`. The setting is `live` and a Job already running keeps its process until the next message starts another.
