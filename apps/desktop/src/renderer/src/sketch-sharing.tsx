// What every sketch in the window shares through: the save dialog main opens, and the toasts that say
// what came of it. A sketch draws no dialog of its own, so this is the one place the preload is reached for it.

import { SketchShareContext, type SketchShareFile, type SketchShareHost } from "@armada/components";
import { useMemo, type ReactNode } from "react";

const bytesOf = (file: SketchShareFile): Uint8Array => (typeof file.data === "string" ? new TextEncoder().encode(file.data) : file.data);

export function SketchSharing({ onCopied, onSaid, children }: { onCopied: (what: string) => void; onSaid: (sentence: string) => void; children: ReactNode }) {
  const host = useMemo<SketchShareHost>(
    () => ({
      save: (files) => window.armada.saveSketchFile(files.map((one) => ({ kind: one.kind, name: one.name, bytes: bytesOf(one) }))),
      copied: onCopied,
      said: onSaid,
    }),
    [onCopied, onSaid],
  );
  return <SketchShareContext.Provider value={host}>{children}</SketchShareContext.Provider>;
}
