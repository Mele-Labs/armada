// The plan's limits on an account off a subscription: the rings load until a Session reports, and on a report
// of no windows they go, leaving Helm at the end of the title bar. Over the `plan-off-subscription` scenario.

import { button, role, walk } from "../walk";

const loading = role("img", "Analyzing Usage");

const planOffSubscription = walk("plan-off-subscription", [
  { look: loading, say: "No Session has reported yet: the rings load" },
  { later: loading, say: "A Session on an enterprise plan reports in a few seconds" },
  { look: button("Helm"), say: "It reported no plan limits, so the rings are gone and Helm ends the title bar" },
]);

export { planOffSubscription as "plan-off-subscription" };
