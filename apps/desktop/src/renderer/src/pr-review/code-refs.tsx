// A changed file named in a finding's words opens in place. **An inline code span is a link only when it is a path the pull request
// changes, or the end of exactly one**, with a line or a run of lines after a colon: `refund_issuer.rb:14` or
// `app/models/payment.rb:14-19`. A press shows those lines of the diff under the words, with Jump to read them in Focus; a second press
// puts them away, and so does Esc. Prose's own reference links draw it (`ProseRefs`), so there is one parser.

import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Button,
  ProseRefs,
  UnifiedDiff,
  type ProseReference,
  type ProseReferences,
} from "@armada/components";
import type { ReviewPlace } from "@armada/protocol";

import { focus } from "./focus-state";
import type { ReviewPr } from "./model";
import { openKind } from "./panes";
import { counted, linesOfPatch, sliceAround } from "./patch";
import { NO_WORK, useReviewStore, useReviews } from "./store";

/** What a span says: a path, and the lines after its colon. */
const SPAN = /^([\w.\-/]+?)(?::(\d+)(?:-(\d+))?)?$/;

/** Where a reference points: the changed file it names, and its lines where it names some. */
export type CodeRef = {
  path: string;
  start?: number;
  end?: number;
};

/** The changed file `name` is, or is the end of. Nothing where it is none of them, or more than one. */
export function changedFileOf(name: string, paths: readonly string[]): string | undefined {
  const matches = paths.filter((one) => one === name || one.endsWith(`/${name}`));
  return matches.length === 1 ? matches[0] : undefined;
}

/** A span read as a reference to a changed file, or nothing. */
export function codeRefOf(span: string, paths: readonly string[]): CodeRef | undefined {
  const said = SPAN.exec(span.trim());
  if (said === null) return undefined;
  const path = changedFileOf(said[1]!, paths);
  if (path === undefined) return undefined;
  if (said[2] === undefined)
    return {
      path,
    };
  const start = Number(said[2]);
  const end = said[3] === undefined ? start : Math.max(start, Number(said[3]));
  return {
    path,
    start,
    end,
  };
}

const idOf = (ref: CodeRef): string =>
  ref.start === undefined
    ? ref.path
    : `${ref.path}:${ref.start}${ref.end === ref.start ? "" : `-${ref.end}`}`;

/** Closes the peek opened last, for Esc. Answers whether one was open. */
const closers: (() => void)[] = [];
export function closeAPeek(): boolean {
  const close = closers.pop();
  close?.();
  return close !== undefined;
}

/** Words whose code spans name changed files, and the lines one of them shows while it is open. */
export function CodeRefs({ pr, children }: { pr: ReviewPr; children: ReactNode }) {
  const [open, setOpen] = useState<CodeRef | undefined>(undefined);
  const paths = useMemo(
    () => pr.page.files.map((one) => one.path),
    [
      pr.page.files,
    ],
  );
  const refs = useMemo(
    (): ProseReferences => ({
      scan: (text) => {
        const ref = codeRefOf(text, paths);
        return ref === undefined
          ? []
          : [
              {
                from: 0,
                to: text.length,
                ref: {
                  kind: "file",
                  id: idOf(ref),
                  title: idOf(ref),
                },
              },
            ];
      },
      open: (reference: ProseReference) =>
        setOpen((was) =>
          was !== undefined && idOf(was) === reference.id
            ? undefined
            : codeRefOf(reference.id, paths),
        ),
      opened: (reference: ProseReference) => open !== undefined && idOf(open) === reference.id,
    }),
    [
      paths,
      open,
    ],
  );
  // Esc closes the one open last.
  useEffect(() => {
    if (open === undefined) return;
    const close = () => setOpen(undefined);
    closers.push(close);
    return () => {
      const at = closers.indexOf(close);
      if (at >= 0) closers.splice(at, 1);
    };
  }, [
    open,
  ]);
  return (
    <ProseRefs.Provider value={refs}>
      {children}
      {open === undefined ? null : <Peek pr={pr} at={open} onJump={() => setOpen(undefined)} />}
    </ProseRefs.Provider>
  );
}

/** The lines a reference names, cut from the file's diff, with Jump. */
function Peek({ pr, at, onJump }: { pr: ReviewPr; at: CodeRef; onJump: () => void }) {
  const store = useReviewStore()!;
  const every = (useReviews().work[pr.key] ?? NO_WORK).every;
  useEffect(
    () => store.readPatches(pr.key),
    [
      store,
      pr.key,
    ],
  );
  const id = idOf(at);
  const patch =
    every.state === "read" ? every.files.find((one) => one.path === at.path) : undefined;
  const lines = patch?.patch === undefined ? [] : linesOfPatch(at.path, patch.patch);
  const place: ReviewPlace | undefined =
    at.start === undefined
      ? undefined
      : {
          path: at.path,
          side: "right",
          start: at.start,
          end: at.end ?? at.start,
        };
  const held =
    place === undefined ||
    counted(lines).some(
      (one) => one.now !== undefined && one.now >= place.start && one.now <= place.end,
    );
  const cut = place === undefined || !held ? lines : sliceAround(lines, place);
  const jump = () => {
    focus.go(
      pr.key,
      at.path,
      at.start === undefined
        ? {}
        : {
            line: {
              line: at.start,
              side: "right",
            },
          },
    );
    openKind(pr.key, "pr-focus");
    onJump();
  };
  return (
    <section className="pr-peek" aria-label={`Lines of ${id}`}>
      <header className="pr-peek__head">
        <span className="pr-peek__path">{id}</span>
        <Button variant="secondary" size="sm" aria-label={`Jump to ${id}`} onClick={jump}>
          Jump
        </Button>
      </header>
      {every.state !== "read" ? (
        <p className="pr-peek__said">{every.state === "said" ? every.said : "Reading the diff."}</p>
      ) : held ? (
        <UnifiedDiff
          bare
          files={[
            {
              path: at.path,
              lines: cut,
            },
          ]}
          emptyNote="No lines changed in this file."
        />
      ) : (
        <p className="pr-peek__said">{`Line ${at.start} is not in the diff.`}</p>
      )}
    </section>
  );
}
