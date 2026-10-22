// Settings → Phone, on the base repository. Open Settings in the rail. The Phone card reads
// `../phone.ts`, which `mount.tsx` resets to the moment named here.

import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";

function board(name: string, says: string): Scenario {
  return { ...onBoard({ repositories: [repository()] }), name, says };
}

export const s207Phone: Scenario = board("phone", "Settings → Phone: access on, two phones paired; Pair a phone shows a code, and a phone claims it");
export const s207PhoneOff: Scenario = board("phone/off", "Settings → Phone with access off: the switch and its line; turning it on shows the two phones");
export const s207PhonePortTaken: Scenario = board("phone/port-taken", "Settings → Phone with access on and port 8443 taken: Fleet's sentence");
export const s207PhoneTailscale: Scenario = board("phone/tailscale", "Settings → Phone with Tailscale signed out");
