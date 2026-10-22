// A Job asks another Job of its repository a question on its Field: an arc runs through the air from its drone to the other's, packets
// travel it toward the receiver and the question is quoted at its middle. The other Job replies and the packets turn round. Then the
// messages go stale and the arc is gone. Over the `job-field-messaging` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const field = region("Field", { exact: true });
const strip = inside(field, role("group", "Drones"));
const main = (now: RegExp) => inside(strip, button(new RegExp(`^Rotate the audit log: ${now.source}`)));
const map = inside(field, role("listbox", "Map"));
const message = (name: RegExp) => inside(map, button(name));

export const jobFieldMessaging = walk("job-field-messaging", [
  { press: deckRow("Rotate the audit log"), say: "The Job is open. The two other live Jobs of its repository are faded on its Field" },
  { later: main(/Thinking$/), say: "Its drone is thinking over its last file. Now it asks Rebuild the store index a question" },
  { look: main(/Messaging Rebuild the store index$/), say: "The Drones menu says who it is messaging" },
  { later: message(/^Message to Rebuild the store index: /), say: "An arc runs from this Job's drone to the other's, which comes up to full strength with its name and beam. Packets travel toward it, and the question is quoted at the arc's middle. A press on the arc or the quote opens that Job. Now it replies" },
  { look: main(/Messaged by Rebuild the store index$/), say: "The Drones menu says who messaged it" },
  { later: message(/^Message from Rebuild the store index: /), say: "The packets now travel toward this Job, and the reply is quoted. Now two and a half minutes pass" },
  { look: main(/Thinking$/), say: "The messages are stale: the arc is gone and the other Job's drone is faded again" },
]);
