// The words the person taught Armada to draw as links, for one repository: what Fleet lists for its root, and no other. A pattern with
// no address for the repository is not listed, so it draws plain there. **Read again whenever a teaching is saved or the Words mods
// change**, because Fleet looks for a repository's address off the request and lists the rule on a later read.

import { useEffect, useState } from "react";
import type { WordRule } from "@armada/protocol";

import { useModList } from "./mod-list";
import { skipsMods } from "./theme-loader";
import { foundSaid, useTeachingsSaved } from "./teach";

/** One pattern to draw as a link, in the form `refScanner` takes. */
export type TaughtLink = { pattern: string; address: string; from?: string };

/** Reads in flight, so the threads of one repository ask Fleet once between them. */
const asking = new Map<string, Promise<readonly WordRule[] | undefined>>();

function ask(root: string, sessionId: string | undefined): Promise<readonly WordRule[] | undefined> {
  const key = `${root}\n${sessionId ?? ""}`;
  let reading = asking.get(key);
  if (reading === undefined) {
    reading = window.armada
      .listWordRules(root, sessionId)
      .then((read) => (read.ok ? read.value.rules : undefined))
      .catch(() => undefined)
      .finally(() => void asking.delete(key));
    asking.set(key, reading);
  }
  return reading;
}

/**
 * The rules that draw words as links in `root`'s Sessions. `sessionId` is the Session the words are drawn in, which Fleet asks in
 * Needs you where it finds no address. Nothing is asked while no Words mod is on.
 */
export function useWordRules(root: string | undefined, sessionId: string | undefined): readonly TaughtLink[] {
  const mods = useModList();
  const saved = useTeachingsSaved();
  // Safe mode reads no mod, so it links nothing a teaching made.
  const words = skipsMods() ? "" : mods.filter((one) => one.kind === "words" && one.valid && one.enabled).map((one) => `${one.name}@${one.changed_at ?? ""}`).join();
  const [read, setRead] = useState<{ root: string; rules: readonly WordRule[] } | undefined>(undefined);
  useEffect(() => {
    if (root === undefined || words === "") return;
    let live = true;
    void ask(root, sessionId).then((rules) => {
      // A read that failed leaves the rules as they were, so a link does not blink out on a dropped connection.
      if (live && rules !== undefined) setRead({ root, rules });
    });
    return () => {
      live = false;
    };
  }, [root, sessionId, words, saved]);
  // Another repository's addresses are never this one's, and a Words mod that is gone leaves nothing to link.
  const rules = root !== undefined && words !== "" && read?.root === root ? read.rules : [];
  return rules.map((one) => ({ pattern: one.pattern, address: one.address, from: foundSaid(one.found) }));
}
