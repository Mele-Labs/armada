// The pull requests waiting on him, in the column. **One dropdown chooses the list**: those someone asked his review of,
// those one of his agents opened, and then every find he saved. A find is what he asked a Drone to go and look for, in his
// own words ("PRs that touch files I've touched"), and its list is what the Drone brought back with the one line that says
// why each matched. **A list is fetched when the view opens and when Refresh is pressed, and never polled**: Armada does
// not watch other people's pull requests, so the glyph on Refresh says when this list was read. A find runs only on send,
// since a Drone looking costs, and again only when he asks for it again.

import { useEffect, useState } from "react";
import { Check, History, LoaderCircle, RotateCw, X } from "lucide-react";
import { Button, DropdownMenu, Input, Kbd, PrMark, SearchField, Textarea, Tooltip, agoOf, forgetKept, sendsOn, useKept } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { parsePullLink } from "../../../shared/pull-link";
import { REVIEW_KEYS, capOf } from "./key-table";
import { openLinked } from "./link";
import { ChecksMark, ReviewMark, RunFailed } from "./marks";
import type { Find, ReviewPr } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";

const LISTS = [
  { id: "requested", label: "Requested for Review" },
  { id: "team", label: "Team" },
  { id: "assigned", label: "Assigned" },
  { id: "mine", label: "My PRs" },
];

const FIND_KEEP = "pr-review:find";
const SEARCH_KEEP = "pr-review:search";
/** The dropdown's last entry: not a list, but the box a find is written in. */
const ASK = "ask";

/** The repository picked narrows the list, as it narrows every other list in the window. */
export const inRepository = (pr: ReviewPr, picked: RepositorySummary | null): boolean => picked === null || picked.manifest === undefined || pr.manifestId === picked.manifest.id;

/** Whether a row answers the words in the bar: its title, `#number`, repository or author, in any case. */
const matches = (pr: ReviewPr, words: string): boolean => {
  const wanted = words.trim().toLowerCase();
  return wanted === "" || [pr.title, `#${pr.number}`, pr.repository, pr.page.author ?? ""].some((one) => one.toLowerCase().includes(wanted));
};

export function ReviewList({ picked, repositories }: { picked: RepositorySummary | null; repositories: readonly RepositorySummary[] }) {
  const store = useReviewStore()!;
  const { prs, tab, finds, open, fetchedAt, fetching, failed, work, searching: asking } = useReviews();
  const setAsking = (on: boolean) => store.setSearching(on);
  const [words, setWords] = useState("");
  const [narrow, setNarrow] = useKept<string>(SEARCH_KEEP, "");
  const [address, setAddress] = useState("");
  const [unopened, setUnopened] = useState<string | undefined>(undefined);
  // The box is drawn only when he chose it in the dropdown, in place of the list, until he sends or leaves it.
  // The list is read each time the view is opened, and on Refresh. Nothing reads it in between.
  useEffect(() => {
    store.fetch();
  }, [store]);

  const find = finds.find((one) => one.id === tab);
  const rows: { pr: ReviewPr; reason?: string }[] =
    find === undefined
      ? prs.filter((one) => one.lists.some((list) => list === tab) && inRepository(one, picked)).map((pr) => ({ pr, ...(tab === "team" && pr.team !== undefined ? { reason: pr.team } : {}) }))
      : find.hits.flatMap((hit) => {
          const pr = prs.find((one) => one.key === hit.key);
          return pr === undefined || !inRepository(pr, picked) ? [] : [{ pr, reason: hit.reason }];
        });
  const shown = rows.filter(({ pr }) => matches(pr, narrow));
  const fetched = fetchedAt === undefined ? "Not fetched" : `Fetched ${agoOf(new Date(fetchedAt).toISOString())}`;
  const here = asking ? ASK : tab;
  const entries: DropdownMenuEntry[] = [
    ...LISTS.map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.label, selected: here === one.id })),
    ...(finds.length === 0 ? [] : [{ kind: "separator", id: "finds-rule" } as const, { kind: "label", id: "finds-label", label: "Finds" } as const]),
    ...finds.map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.words, selected: here === one.id })),
    { kind: "separator", id: "ask-rule" },
    { kind: "item", id: ASK, label: "Natural Search", selected: asking, shortcut: capOf(REVIEW_KEYS.search) },
  ];
  const ask = () => {
    if (words.trim() === "") return;
    const said = words;
    // The words stay in the box until he has said Run, so Cancel loses nothing.
    store.spend("Natural Search", () => {
      store.find(said);
      forgetKept(FIND_KEEP);
      setWords("");
      setAsking(false);
    });
  };

  // A link opens whichever pull request it names, on a list or not. A refusal is said under the box and the link stays for him to correct.
  const openLink = async () => {
    const link = parsePullLink(address);
    if (link === undefined) return setUnopened("That is not a link to a pull request.");
    const refused = await openLinked(store, repositories, link);
    if (refused !== undefined) return setUnopened(refused);
    setUnopened(undefined);
    setAddress("");
    setAsking(false);
  };

  return (
    <section className="deck-panel deck-list pr-list" aria-label="Pull requests" tabIndex={-1}>
      <header className="deck-band">
        Pull requests
        <Tooltip label="Go to Pull requests" shortcut={REVIEW_KEYS.list}>
          <Kbd>{REVIEW_KEYS.list}</Kbd>
        </Tooltip>
        <span className="pr-list__refresh">
          <Tooltip label={fetching ? "Fetching" : fetched}>
            <Button variant="ghost" size="sm" iconOnly aria-label="Refresh" disabled={fetching} onClick={() => store.fetch()}>
              <RotateCw size={16} strokeWidth={2} aria-hidden data-fetching={fetching || undefined} className="pr-list__spin" />
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="pr-list__search">
        <SearchField label="Search the pull requests" value={narrow} onValue={setNarrow} cap={REVIEW_KEYS.filter} />
      </div>
      <div className="deck-scope pr-list__pick">
        <DropdownMenu
          align="start"
          portal
          triggerLabel={asking ? "Natural Search" : find === undefined ? (LISTS.find((one) => one.id === tab)?.label ?? "") : find.words}
          triggerAriaLabel="List"
          entries={entries}
          onSelect={(id) => {
            setAsking(id === ASK);
            if (id !== ASK) store.setTab(id);
          }}
        />
        <Tooltip label="Open the lists" shortcut={capOf(REVIEW_KEYS.lists)}>
          <Kbd>{capOf(REVIEW_KEYS.lists)}</Kbd>
        </Tooltip>
      </div>
      <form
        className="pr-link"
        aria-label="Open a pull request"
        onSubmit={(event) => {
          event.preventDefault();
          void openLink();
        }}
      >
        <Input
          aria-label="Pull request link"
          placeholder="Paste a pull request link"
          mono
          value={address}
          invalid={unopened !== undefined}
          {...(unopened === undefined ? {} : { message: unopened })}
          onChange={(event) => {
            setAddress(event.target.value);
            setUnopened(undefined);
          }}
        />
      </form>
      {asking ? (
      <form
        className="pr-find"
        aria-label="Natural Search"
        onSubmit={(event) => {
          event.preventDefault();
          ask();
        }}
      >
        <Textarea
          rows={3}
          autoFocus
          keep={FIND_KEEP}
          aria-label="Natural Search"
          value={words}
          onChange={(event) => setWords(event.target.value)}
          onKeyDown={(event) => {
            if (sendsOn(event) && !event.nativeEvent.isComposing) {
              event.preventDefault();
              ask();
            }
          }}
        />
        <Tooltip label="A Drone looks for them, once, when you send" asChild>
          <Button variant="secondary" size="sm" type="submit" disabled={words.trim() === ""}>
            Search
          </Button>
        </Tooltip>
      </form>
      ) : (
        <>
      {failed === undefined ? null : (
        <p className="pr-list__failed" role="alert">
          {failed}
        </p>
      )}
      {find === undefined ? null : <FindHead find={find} />}
      <ul className="deck-list__rows pr-list__rows">
        {shown.map(({ pr, reason }) => (
          <li key={pr.key} className="deck-list__item">
            <button type="button" className="pr-row" data-selected={open === pr.key || undefined} aria-current={open === pr.key || undefined} aria-label={`#${pr.number} ${pr.title}`} onClick={() => store.open(pr.key)}>
              <span className="pr-row__title">{pr.title}</span>
              <ReviewMark review={work[pr.key]?.review ?? NO_WORK.review} />
              <ChecksMark checks={pr.checks} />
              <span className="pr-row__facts">
                <span className="pr-row__number mono">{`#${pr.number}`}</span>
                <span className="pr-row__author">{pr.page.author}</span>
                <span className="pr-row__repository">{pr.repository}</span>
              </span>
              {reason === undefined ? null : (
                <Tooltip label={reason} asChild>
                  <span className="pr-row__reason" role="note" aria-label={reason}>
                    {reason}
                  </span>
                </Tooltip>
              )}
            </button>
          </li>
        ))}
      </ul>
        </>
      )}
    </section>
  );
}

/** What a saved find says of itself: running, or read, with the two acts on it. */
function FindHead({ find }: { find: Find }) {
  const store = useReviewStore()!;
  const running = find.run.state === "running";
  return (
    <div className="pr-find__head">
      {running ? (
        <span className="pr-review__mark" data-checks="running">
          <PrMark Glyph={LoaderCircle} said="Looking" tone="running" />
        </span>
      ) : find.run.state === "failed" ? (
        <RunFailed said={find.run.said} />
      ) : find.run.state === "done" && find.hits.length === 0 ? (
        <PrMark Glyph={Check} said="Nothing found" tone="success" />
      ) : null}
      {find.run.state === "done" ? <PrMark Glyph={History} said={`Kept. Ran ${agoOf(new Date(find.run.at).toISOString())}`} /> : null}
      <span className="pr-find__acts">
        <Tooltip label="Run again">
          <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={running} onClick={() => store.spend("Natural Search", () => store.rerunFind(find.id))}>
            <RotateCw size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Remove this find">
          <Button variant="ghost" size="sm" iconOnly aria-label="Remove this find" disabled={running} onClick={() => store.removeFind(find.id)}>
            <X size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      </span>
    </div>
  );
}
