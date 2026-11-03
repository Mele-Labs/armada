// What a page's ask is, as a Needs you item: its question and its two options. The app's Fleet is told them (`main/job-browsers.ts`) and the
// mock puts the same ones on a Job, so the two read alike.

import type { WaitingOption } from "@armada/protocol";

import { labelOfPermission, type SitePermission } from "./site-permissions";

/** What Local network reaches, said under its Allow. */
export const LOCAL_NETWORK_RISK = "It can reach apps on this Mac and devices on your network.";

/** The question: one permission, and the site that wants it. */
export const browserAskText = (permission: SitePermission, host: string): string =>
  `Allow ${labelOfPermission(permission)} for ${host}?`;

/** Allow and Block, in that order: the index Fleet answers with is 0 for Allow and 1 for Block. */
export const browserAskOptions = (permission: SitePermission): WaitingOption[] => [
  {
    label: "Allow",
    ...(permission === "local_network"
      ? {
          description: LOCAL_NETWORK_RISK,
        }
      : {}),
  },
  {
    label: "Block",
  },
];
