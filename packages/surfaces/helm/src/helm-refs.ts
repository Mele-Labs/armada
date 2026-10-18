// What Helm's words name that Bridge can open: a pull request or an issue by its number, a Session by its id, its address
// or its exact title. **Read off what the window holds**, never off the words' own claim: a number or a name that matches nothing here is left as text.
// The spans are `Prose`'s to turn into links (`ProseRefs`); this is the finding.

import type { ProseReference } from "@armada/components";
import type { MergeLine } from "@armada/protocol";

/** What there is to find. `numbered` is the conversation's repository: its open issues and pull requests, and any number already looked up. */
export type RefWorld = {
  sessions: readonly { id: string; title?: string }[];
  /** `url` is the forge's address for it, which the same number in another repository does not share. */
  numbered: readonly { kind: "issue" | "pull_request"; number: string; title: string; url?: string }[];
  /** `owner/name` of the repository the words are in, read off the forge addresses it holds. Without it `owner/name#12` is always another repository's. */
  slug?: string;
};

export type RefSpan = { from: number; to: number; ref: ProseReference };

const escaped = (word: string): string => word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Every place `needle` stands as a whole word, so `s-01J9ZK7M` is not found inside a longer word. */
function* whole(text: string, needle: string): Generator<[number, number]> {
  if (needle === "") return;
  for (const found of text.matchAll(new RegExp(`(?<![\\w-])${escaped(needle)}(?![\\w-])`, "g"))) yield [found.index, found.index + needle.length];
}

/**
 * The branch a pull request is on, read off the repository's merge line: the open ones Fleet read off the forge,
 * the newest merged, and the rows of the line itself. **Nothing where the line has no row for it**, and the Merge
 * line then opens with nothing marked.
 */
export function branchOfPullRequest(line: MergeLine | undefined, number: string): string | undefined {
  const at = Number(number);
  return (
    line?.hub?.pull_requests?.find((one) => one.number === at)?.branch ??
    line?.hub?.merged?.find((one) => one.number === at)?.branch ??
    [...(line?.line ?? []), ...(line?.landed ?? []), ...(line?.sent_back ?? [])].find((one) => one.pull_request?.number === at)?.branch
  );
}

/** `owner/name` out of the first forge address of a pull request or an issue that names one. */
export function forgeSlug(addresses: readonly (string | undefined)[]): string | undefined {
  for (const one of addresses) {
    const slug = one === undefined ? undefined : /^https:\/\/github\.com\/([\w.-]+\/[\w.-]+)\/(?:pull|issues)\/\d+/.exec(one)?.[1];
    if (slug !== undefined) return slug;
  }
  return undefined;
}

/** Another repository's number: its id is `owner/name#12`, which `forgeAddress` turns back into the forge's page. */
export const isElsewhere = (id: string): boolean => /^[\w.-]+\/[\w.-]+#\d+$/.test(id);

/** The forge's page for a reference to another repository. An issue's address reaches a pull request too. */
export function forgeAddress(ref: ProseReference): string {
  const [slug = "", number = ""] = ref.id.split("#");
  return `https://github.com/${slug}/${ref.kind === "pull_request" ? "pull" : "issues"}/${number}`;
}

/** The title without the `#2214 ` a tag's title leads with. */
const bare = (title: string): string => title.replace(/^#\d+\s*/, "");

/** Spans of `text` that name something, longest first where two overlap. */
export function scanRefs(text: string, world: RefWorld): RefSpan[] {
  const found: RefSpan[] = [];
  const local = (number: string) => world.numbered.find((it) => it.number === number);
  const same = (slug: string) => world.slug !== undefined && world.slug.toLowerCase() === slug.toLowerCase();
  const elsewhere = (kind: "issue" | "pull_request", slug: string, number: string): ProseReference => ({ kind, id: `${slug}#${number}`, title: `${slug}#${number}` });
  for (const one of text.matchAll(/(?<![\w&/#])#(\d+)(?!\w)/g)) {
    const hit = local(one[1]!);
    if (hit !== undefined) found.push({ from: one.index, to: one.index + one[0].length, ref: { kind: hit.kind, id: hit.number, title: bare(hit.title) } });
  }
  // `owner/name#12`: this repository's own number when it names this one, else a page on the forge.
  for (const one of text.matchAll(/(?<![\w./-])([\w.-]+\/[\w.-]+)#(\d+)(?!\w)/g)) {
    const hit = same(one[1]!) ? local(one[2]!) : undefined;
    const ref: ProseReference = hit !== undefined ? { kind: hit.kind, id: hit.number, title: bare(hit.title) } : elsewhere("issue", one[1]!, one[2]!);
    found.push({ from: one.index, to: one.index + one[0].length, ref });
  }
  // The forge's own address for one of them, whole. One this repository holds is its own number; any other is the forge's page.
  for (const one of text.matchAll(/(?<![(\[<\w])(https?:\/\/[^\s<>`)\]]+?)\/(?:pull|issues)\/(\d+)(?![\w/#?])/g)) {
    const hit = world.numbered.find((it) => it.number === one[2] && it.url !== undefined && it.url === one[0]);
    const github = /^https:\/\/github\.com\/([\w.-]+\/[\w.-]+)\/(pull|issues)\/(\d+)$/.exec(one[0]);
    const span = { from: one.index, to: one.index + one[0].length };
    if (hit !== undefined) found.push({ ...span, ref: { kind: hit.kind, id: hit.number, title: bare(hit.title) } });
    else if (github !== null) found.push({ ...span, ref: elsewhere(github[2] === "pull" ? "pull_request" : "issue", github[1]!, github[3]!) });
  }
  for (const session of world.sessions) {
    const ref: ProseReference = { kind: "session", id: session.id, title: session.title ?? session.id };
    const prefix = session.id.slice(0, 8);
    for (const needle of [session.id, `s-${prefix}`, prefix, ...(session.title === undefined || session.title.length < 4 ? [] : [session.title])])
      for (const [from, to] of whole(text, needle)) found.push({ from, to, ref });
  }
  const kept: RefSpan[] = [];
  for (const one of found.sort((a, b) => b.to - b.from - (a.to - a.from) || a.from - b.from)) {
    if (!kept.some((it) => one.from < it.to && it.from < one.to)) kept.push(one);
  }
  return kept.sort((a, b) => a.from - b.from);
}
