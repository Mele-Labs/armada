// Where Helm's dock says the person is — #1075. No window, no Fleet: the two
// decisions a rendering cannot show on its own, folded here and tested on
// their own.

import type { HelmContext, HelmScreen } from "@armada/protocol";

/** Which screen `AskHelm.context` names, in the precedence `App.tsx` draws by. */
export function screenOf(where: {
  clearing: boolean;
  manifesting: boolean;
  studying: boolean;
  kitting: boolean;
  settling: boolean;
  /** The Workflow creator. Optional, since only the window itself draws it. */
  workflowing?: boolean;
}): HelmScreen {
  if (where.clearing) return "cleanup";
  if (where.manifesting) return "manifest";
  if (where.studying) return "studio";
  if (where.kitting) return "kit";
  if (where.workflowing === true) return "workflows";
  // **Last, and it was missing entirely until #1275.** A person on Settings
  // was told to Helm as being on the Board, which is the gap #1287 left when
  // it added `studio` and stopped.
  if (where.settling) return "settings";
  // **Overview, because it is where a window with nothing else open is.**
  // Everything else — the composer, a Session, the reports — is Overview with
  // something over it.
  return "overview";
}

/**
 * `AskHelm.context`, sent with every ask. Absent fields are left off the wire rather than sent `null`.
 * `studio` and `node` are the Studios surface's own — #1287 — and a node is sent only beside its Studio.
 */
export function contextOf(where: {
  screen: HelmScreen;
  picked: string | null;
  studio?: string | null;
  node?: string | null;
}): HelmContext {
  const studio = where.screen === "studio" ? (where.studio ?? null) : null;
  const node = studio === null ? null : (where.node ?? null);
  return {
    screen: where.screen,
    ...(where.picked !== null ? { picked: where.picked } : {}),
    ...(studio !== null ? { studio } : {}),
    ...(node !== null ? { node } : {}),
  };
}

/** How the open Studio and its selected node are named on the footer — `App.tsx` resolves the ids. */
export type StudioNamed = { name: string; node?: string };

const SCREEN_LABEL: Record<HelmScreen, string> = {
  overview: "Cockpit",
  manifest: "Manifest",
  cleanup: "Cleanup",
  studio: "Studios",
  kit: "Kit",
  settings: "Settings",
  workflows: "Workflows",
};

/**
 * The composer footer's one sentence — #1094. **Read off `context` itself**,
 * the same shape `contextOf` just built for the wire, so the words on screen
 * can never name a screen or a row other than the one actually sent.
 */
export function locationOf(context: HelmContext, studio?: StudioNamed): string {
  const label = SCREEN_LABEL[context.screen];
  // A Studio names itself, and the node selected on it, only where the wire carries the Studio.
  if (context.screen === "studio") {
    if (context.studio === undefined || studio === undefined) return label;
    const on = context.node === undefined || studio.node === undefined ? "" : ` · ${studio.node} selected`;
    return `${label} · ${studio.name}${on}`;
  }
  return label;
}
