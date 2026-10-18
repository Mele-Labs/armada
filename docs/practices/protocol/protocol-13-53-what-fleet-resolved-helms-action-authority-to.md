# Protocol 13.53: what Fleet resolved Helm's action authority to

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`FleetHealth.helm_action_authority`, additive (#1127).
`settings.helm-action-authority-tier-1-redirect-enabled-vs-read-only` resolves once when Fleet starts, and until now nothing on the wire carried the answer — Settings' "This machine" section could only describe what the setting does, not say what Fleet actually decided.
`GET /health` answers it now, alongside the probes it already carried.
