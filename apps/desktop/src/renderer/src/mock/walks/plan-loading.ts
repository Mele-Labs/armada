// The plan's limits ring while Fleet has no reading: two rings with an arc sweeping each, in the ring's place,
// then the drawn ring once a reading arrives. Over the `plan-loading` scenario.

import { role, walk } from "../walk";

const loading = role("img", "Plan limits: no reading yet");

const planLoading = walk("plan-loading", [
  { hover: loading, say: "No reading yet: an arc sweeps each ring. Hover for its name" },
  { later: loading, say: "A reading arrives in a few seconds" },
  { look: role("button", /5-hour limit 12%/), say: "The drawn ring: 5 hours 12%, the week 87%" },
]);

export { planLoading as "plan-loading" };
