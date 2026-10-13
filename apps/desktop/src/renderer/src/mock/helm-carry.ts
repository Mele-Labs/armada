// What Helm's box carries in the mock, as Fleet keeps it: the `/` list, the files an ask carried, and
// rows added to Helm's thread. Outside `scenarios/`, where every export is read as a row.

import type { HelmBoxFacts, HelmThreadItem, SessionTag, SessionUpload } from "@armada/protocol";

import type { FleetHandle } from "./moment";

/** What `/` offers in a message box: the agent's own skills and commands, as its init line names them. */
export const COMMANDS = ["review", "simplify", "security-review", "init", "compact", "clear", "model"];

/** What Helm's box offers in this scenario: no model or effort picked, and the same commands a Session lists. */
export const HELM_FACTS: HelmBoxFacts = { commands: COMMANDS };

/** The files Helm's messages carried, by conversation and id, as Fleet keeps them for `get_helm_file`. */
export const KEPT_FILES = new Map<string, { bytes: Uint8Array; type: string }>();
let fileCounter = 0;

/** What an ask carried, as the `asked` row of the thread keeps it: each upload kept, and the tags as sent. */
export function carriedBy(manifestId: string, carry: { attachments?: SessionUpload[]; mentions?: SessionTag[] } | undefined): Pick<Extract<HelmThreadItem, { kind: "asked" }>, "files" | "tags"> {
  const files = (carry?.attachments ?? []).map((upload) => {
    fileCounter += 1;
    const id = `file-${fileCounter}`;
    KEPT_FILES.set(`${manifestId}\n${id}`, { bytes: Uint8Array.from(atob(upload.data), (one) => one.charCodeAt(0)), type: upload.media_type });
    return { id, name: upload.name, media_type: upload.media_type };
  });
  const tags = carry?.mentions ?? [];
  return { ...(files.length === 0 ? {} : { files }), ...(tags.length === 0 ? {} : { tags }) };
}

/** Items added to Helm's thread, after the ones it holds: each takes the next id. */
export function appended(
  handle: FleetHandle,
  manifestId: string,
  ...items: ((id: string) => HelmThreadItem)[]
): void {
  const held = handle.state().helm;
  const was = held.state === "open" ? held.items : [];
  handle.publish({
    helm: {
      state: "open",
      manifestId,
      replying: false,
      skipped: 0,
      missed: 0,
      items: [...was, ...items.map((item, at) => item(String(was.length + at)))],
      facts: held.state === "open" ? held.facts : HELM_FACTS,
    },
  });
}
