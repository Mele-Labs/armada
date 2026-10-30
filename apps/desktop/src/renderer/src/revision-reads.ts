// The files a Job kept a revision in, each read once as it arrives: a document's pages, a sketch's redraws.

import { useEffect, useMemo, useRef, useState } from "react";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

/**
 * Each path's file as `read` makes it, in the order given. **A file not read yet, or one `read` refuses, is `undefined`.**
 * `read` is held at its first value, so pass one declared outside the component.
 */
export function useRevisionReads<T>(draft: SessionsDraft | undefined, session: string, paths: readonly string[], read: (text: string) => T | undefined): (T | undefined)[] {
  const cache = useRef(new Map<string, T>());
  const reader = useRef(read);
  const [version, setVersion] = useState(0);
  const key = paths.join("|");
  useEffect(() => {
    let live = true;
    const missing = key.split("|").filter((one) => one !== "" && !cache.current.has(one));
    if (missing.length === 0) return undefined;
    void Promise.all(
      missing.map(async (path) => {
        const got = await draft?.readArtifact?.(session, path);
        const made = got?.ok === true ? reader.current(new TextDecoder().decode(got.bytes)) : undefined;
        if (made !== undefined) cache.current.set(path, made);
      }),
    ).then(() => {
      if (live) setVersion((was) => was + 1);
    });
    return () => {
      live = false;
    };
  }, [draft, session, key]);
  return useMemo(() => (key === "" ? [] : key.split("|").map((one) => cache.current.get(one))), [key, version]);
}
