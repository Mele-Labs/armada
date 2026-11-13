// His other Macs: one on the Devices list, pairing it, and what its Phone Gateway hands this Fleet once paired.
// Hand-mirrored from `crates/ipc/src/macs.rs` and `Mac` in `crates/ipc/src/phone.rs`.
//
// The header rules in `protocol.ts` hold here. `docs/practices/protocol/his-other-macs-fleets-half.md`.

import type { BugReport } from "./bug-reports";
import type { Event } from "./events";
import type { Project } from "./projects";
import type { SessionRecord } from "./sessions";

/** Each Mac's one colour, on its chip wherever its Jobs are drawn. */
export type MacColour = "blue" | "teal" | "green" | "orange";

/** How the link to another Mac stands. */
export type MacLink =
  | {
      state: "linked";
    }
  /** Not answering since `since` (Unix seconds). What is sent to it waits here. */
  | {
      state: "asleep";
      since: number;
    }
  /** It runs an older build than this Mac, so the link is frozen until it updates. */
  | {
      state: "older_build";
    }
  | {
      state: "newer_build";
    }
  /** It claimed a pairing link and waits for Confirm on this Mac. */
  | {
      state: "claim_waiting";
    }
  /** This Mac claimed its link and waits for Confirm on it. `why` names the Mac to press it on. */
  | {
      state: "confirm_waiting";
    };

/** A Mac on the Devices list: this one, or one paired with it, or one whose pairing waits on a Confirm. */
export type Mac = {
  /** Minted once by its own Fleet. */
  id: string;
  name: string;
  colour: MacColour;
  /** Its way in, where it has one. */
  address?: string;
  this_mac: boolean;
  /** The commit its Fleet was built from, where it says. */
  build?: string;
  /** How the link to it stands. Absent on this Mac. */
  link?: MacLink;
  /** When this Mac last heard from it (Unix seconds). Absent on this Mac and before the first answer. */
  last_heard_at?: number;
  /** Why the link is stuck, or where the next press is, as a sentence the screen shows as it is. */
  why?: string;
  /** The four digits both Macs show while a pairing asked from Settings waits for Confirm. Absent on a pairing by link. */
  code?: string;
};

/** `POST /admin/macs/pair/start` on the Gateway: the one-time link the other Mac's Settings takes. */
export type MacPairLink = {
  link: string;
  /** Unix seconds. After this the link is refused, and pairing is started again. */
  expires_at: number;
};

/** `POST /admin/macs/pair/claim`: a link pasted from the other Mac. */
export type ClaimMacLink = {
  link: string;
};

/** `POST /admin/macs/pair/confirm` and `POST /admin/macs/forget`: the Mac pressed on. Fleet's `/other_macs/*` answer it too. */
export type MacNamed = {
  mac: string;
};

/** `PUT /other_macs/sessions` on Fleet: every Job row Mac `mac` shares, replacing all this Fleet held from it. */
export type OtherMacSessions = {
  mac: string;
  sessions: SessionRecord[];
};

/** `PUT /other_macs/bug_reports` on Fleet: every report Mac `mac` shares, replacing all this Fleet held from it. */
export type OtherMacBugReports = {
  mac: string;
  reports: BugReport[];
};

/** `PUT /other_macs/projects` on Fleet: every project Mac `mac` shares, with its tasks, replacing all this Fleet held from it. */
export type OtherMacProjects = {
  mac: string;
  projects: Project[];
};

/** `POST /other_macs/event` on Fleet: one event that crossed from Mac `mac`, for this Fleet to keep and publish. */
export type OtherMacEvent = {
  mac: string;
  event: Event;
};
