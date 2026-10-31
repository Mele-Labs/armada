// What a Job browser's page is given when it asks, from the person's saved choices. Pure: `job-browsers.ts` supplies the rules.

import { effectiveChoice, type Answer, type Choice, type SitePermission, type SiteSettings } from "../../shared/site-permissions";

type Details = { mediaTypes?: readonly string[] };

/** The permissions an Electron request names, or null for one that is always refused. Electron sends "local-network-access", which its typed union omits. */
export function permissionsOf(permission: string, details: Details): SitePermission[] | null {
  switch (permission) {
    case "local-network-access":
      return ["local_network"];
    case "media": {
      const types = details.mediaTypes ?? [];
      const wants: SitePermission[] = [];
      if (types.includes("video")) wants.push("camera");
      if (types.includes("audio")) wants.push("microphone");
      return wants.length === 0 ? ["camera", "microphone"] : wants;
    }
    case "notifications":
      return ["notifications"];
    case "clipboard-read":
    case "clipboard-sanitized-write":
      return ["clipboard"];
    case "geolocation":
      return ["location"];
    default:
      return null;
  }
}

export type Rules = {
  settings(): SiteSettings;
  driving(): boolean;
  ask(permission: SitePermission, host: string): Promise<Answer>;
  remember(host: string, permission: SitePermission, choice: Choice): void;
};

/** Whether a request is given: **never while the agent drives**, and asked of the person where the choice is Ask. */
export async function decide(wants: SitePermission[], host: string, rules: Rules): Promise<boolean> {
  if (!rules.driving()) return false;
  for (const want of wants) {
    const choice = effectiveChoice(rules.settings(), host, want);
    if (choice === "block") return false;
    if (choice === "allow") continue;
    const answer = await rules.ask(want, host);
    // The owner may have handed the page back while the question stood.
    if (answer === "dismiss" || !rules.driving()) return false;
    rules.remember(host, want, answer);
    if (answer === "block") return false;
  }
  return true;
}

/** The synchronous check Electron makes: given only what is already allowed, and never asks. */
export function check(wants: SitePermission[], host: string, rules: Rules): boolean {
  return rules.driving() && wants.every((want) => effectiveChoice(rules.settings(), host, want) === "allow");
}

export function downloadOf(host: string, rules: Rules): Choice {
  return rules.driving() ? effectiveChoice(rules.settings(), host, "downloads") : "block";
}

/** The host of a web address, or null for anything else. */
export function hostOf(address: string): string | null {
  try {
    const url = new URL(address);
    return url.protocol === "http:" || url.protocol === "https:" ? url.host : null;
  } catch {
    return null;
  }
}
