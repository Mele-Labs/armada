# Protocol 13.40: how many of a step's Checks run at once

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`LimitValues.checks_at_once` and `SaveLimits.checks_at_once`, additive: a fourth limit on `get_limits` and `save_limits`, from 1 to 8 (#284).
Fleet runs a step's Checks up to it, and before starting each one after the first reads the machine against the memory and disk limits, so a short machine makes the next Check wait for a running one to finish.
