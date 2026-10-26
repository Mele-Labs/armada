// Claude signed out for the whole Fleet: one alert with a Sign in act. Pressed, it shows the sign-in under
// way, then goes. Over `agent-signed-out`.

import { button, deckRow, inside, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const alert = role("status");

const agentSignedOut = walk("agent-signed-out", [
  toSessions,
  { look: inside(alert, text("Claude is signed out")), say: "Claude is signed out" },
  { press: inside(alert, button("Sign in", { exact: true })), say: "Sign in" },
  { look: inside(alert, role("img", "Signing in")), say: "Signing in" },
  { look: deckRow(/Cart totals rounding/), say: "The alert is gone" },
]);

export { agentSignedOut as "agent-signed-out" };
