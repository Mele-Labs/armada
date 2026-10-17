// Settings → Phone, on the base repository. Open Settings in the rail. The Phone card reads
// `../phone.ts`, which `mount.tsx` resets to the moment named here.

import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";

function board(name: string, says: string): Scenario {
  return { ...onBoard({ repositories: [repository()] }), name, says };
}

export const s207Phone: Scenario = board("phone", "Settings → Phone: two phones paired; Pair a phone shows a code, and a phone claims it");
export const s207PhoneGatewayDown: Scenario = board("phone/gateway-down", "Settings → Phone with the Gateway not running");
export const s207PhoneTailscale: Scenario = board("phone/tailscale", "Settings → Phone with Tailscale signed out");
