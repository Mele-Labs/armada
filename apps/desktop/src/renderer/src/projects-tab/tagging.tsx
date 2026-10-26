// What a task is written with: the request box's `#` and `@` menus (`useAtMenu`, `SessionMenu`) on the projects' own
// fields. `#` offers the repository's issues and pull requests, `@` its live Jobs. On Enter each tag the words still
// carry becomes a link on the task, and its words leave the title: the card draws the link as a chip.

import { useEffect, useId, useMemo, useRef, type ChangeEvent, type SyntheticEvent } from "react";
import type { ComposerNumbered, ComposerTag, NumberedHit } from "@armada/components";

import { SessionMenu, useAtMenu, type Offered } from "../dispatch-panel/SessionMenu";
import { jobTitle } from "../issues-tab/marks";
import { useIssues, useIssuesStore } from "../issues-tab/store";
import { issueHit, pullRequestHit, useNumbered } from "../numbered";
import { useSessions } from "../sessions-draft";
import type { Linked, LinkedPull, TaskLinks } from "./model";
import { useProjectsStore } from "./store";

/** The open issues the repository's tracker lists, read as the Issues tab reads them, and what it said where the read failed. */
export function useOpenIssues(manifestId: string): { issues: readonly Linked[]; failed?: string } {
  const issues = useIssuesStore();
  const { trackers } = useIssues();
  const tracker = trackers?.find((one) => one.manifestId === manifestId);
  const query = tracker?.state === "read" ? tracker.open : undefined;
  useEffect(() => issues?.readTrackers([manifestId]), [issues, manifestId]);
  useEffect(() => {
    if (query !== undefined) issues?.run(manifestId, query);
  }, [issues, manifestId, query]);
  const run = query === undefined ? undefined : issues?.runOf(manifestId, query);
  if (run?.state === "failed") return { issues: [], failed: run.said };
  return { issues: run?.state === "read" ? run.tickets.map((one) => ({ reference: one.reference, title: one.title, url: one.url, state: one.state })) : [] };
}

const said = (title: string, reference: string): string => (title.startsWith(`${reference} `) ? title.slice(reference.length + 1) : title);
const referenceOf = (tag: ComposerTag): string => (/^\d+$/.test(tag.id) ? `#${tag.id}` : tag.id);

export function useTagField(manifestId: string, words: string, onWords: (next: string) => void) {
  const store = useProjectsStore()!;
  const { issues } = useOpenIssues(manifestId);
  const elsewhere = useNumbered(manifestId);
  const sessions: Offered[] = useSessions()
    .filter((one) => one.manifestId === manifestId && one.dead === undefined)
    .map((one) => ({ id: one.id, title: jobTitle(one) }));
  const pulls = store.pullsOf(manifestId);
  const numbered = useMemo((): ComposerNumbered => {
    const own: NumberedHit[] = [
      ...issues.map((one) => issueHit(one)),
      ...pulls.map((one): NumberedHit => {
        const { tag } = pullRequestHit(one);
        return one.state === "merged" ? { tag, standing: "Merged" } : one.state === "closed" ? { tag, standing: "Closed" } : { tag };
      }),
    ];
    const held = new Set(own.map((one) => `${one.tag.kind}:${one.tag.id}`));
    return {
      open: [...own, ...(elsewhere?.open ?? []).filter((one) => !held.has(`${one.tag.kind}:${one.tag.id}`))],
      find: elsewhere?.find ?? (() => Promise.resolve(undefined)),
    };
  }, [issues, pulls, elsewhere]);
  // Tags looked up by number are not among the open ones, so each chosen is kept until the words are sent.
  const chosen = useRef<ComposerTag[]>([]);
  // The words last written, ahead of the field: a pick writes them before they are drawn, and a caret read off the old text
  // in that gap would open the menu again over them.
  const latest = useRef(words);
  const write = (next: string) => {
    latest.current = next;
    onWords(next);
  };
  const at = useAtMenu(words, write, { sessions, numbered, onTag: (tag) => void chosen.current.push(tag) }, `projects-tag-${useId()}`);

  /** The words as a title, and the links their tags name. A tag typed by number, not picked, is read the same. */
  const resolve = (text: string): { title: string; links: TaskLinks } => {
    let title = ` ${text} `;
    const issueLinks: Linked[] = [];
    const pullLinks: LinkedPull[] = [];
    const known = [...chosen.current, ...numbered.open.map((one) => one.tag)];
    for (const tag of known) {
      const reference = referenceOf(tag);
      const whole = title.includes(` ${tag.title} `) ? tag.title : title.includes(` ${reference} `) ? reference : undefined;
      if (whole === undefined) continue;
      title = title.replace(` ${whole} `, " ");
      if (tag.kind === "issue" && !issueLinks.some((one) => one.reference === reference)) {
        const open = issues.find((one) => one.reference === reference);
        issueLinks.push(open ?? { reference, title: said(tag.title, reference), url: tag.url ?? "", state: "open" });
      } else if (tag.kind === "pull_request" && !pullLinks.some((one) => one.number === Number(tag.id))) {
        const served = pulls.find((one) => one.number === Number(tag.id));
        pullLinks.push(served ?? { number: Number(tag.id), title: said(tag.title, reference), url: tag.url ?? "", state: "open" });
      }
    }
    // The longest title first, so one Job's title inside another's is not taken for it.
    const job = [...sessions].sort((a, b) => b.title.length - a.title.length).find((one) => title.includes(`@${one.title}`));
    if (job !== undefined) title = title.replace(`@${job.title}`, " ");
    chosen.current = [];
    const left = title.replace(/\s+/g, " ").trim();
    return {
      title: left !== "" ? left : (issueLinks[0]?.title ?? pullLinks[0]?.title ?? job?.title ?? ""),
      links: { ...(job === undefined ? {} : { job: { id: job.id, title: job.title } }), ...(issueLinks.length === 0 ? {} : { issues: issueLinks }), ...(pullLinks.length === 0 ? {} : { pulls: pullLinks }) },
    };
  };

  return {
    resolve,
    /** On the field: the menu reads its words, its caret and its keys. `onKeyDown` is true where the key was the menu's. */
    field: {
      "aria-haspopup": "listbox" as const,
      "aria-expanded": at.open,
      "aria-controls": at.open ? at.listId : undefined,
      "aria-activedescendant": at.activeId,
      onChange: (event: ChangeEvent<HTMLTextAreaElement>) => {
        write(event.target.value);
        at.onChange(event);
      },
      onSelect: (event: SyntheticEvent<HTMLTextAreaElement>) => {
        if (event.currentTarget.value === latest.current) at.onSelect(event);
      },
      onKeyDown: at.onKeyDown,
      onBlur: at.onBlur,
    },
    menu: !at.open ? null : <SessionMenu id={at.listId} label={at.label} numbered={at.numbered} rows={at.offered} active={at.active} onHover={at.setActive} onChoose={at.choose} />,
  };
}
