// A pull request in the side panel: the panel a Job opens, filled from the forge's read, so it is the same for a pull request a
// Job holds and one nobody does. **What only a Job can do is drawn only when one holds it**: sending its checks back to it, and
// its name in the head. Everything else is the forge's.

import { useEffect, useMemo, useRef, useState } from "react";
import { Alert, PullRequestPanel, forgetKept, mayWrite, repositoryOfAddress } from "@armada/components";
import type { FilesReach, PullRequestComposerProps, ReachTest } from "@armada/components";
import { placeOf, whyNotOpenedLink } from "@armada/screens";
import { useAtFloor } from "@armada/shell";
import type { PullReviewRow, PullRequestPage } from "@armada/protocol";
import type { PullRequestAct, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { openLink } from "../commands";
import { PrOwnersOnTheWire } from "../pr-owners-wire";
import { useSessions, useSessionsDraft, useSessionsSaid } from "../sessions-draft";
import { failingOf, promptsOut, useSendBack } from "../session-pull-request";
import { useBack } from "./panels";
import { ReviewTab } from "./review-tab";
import { useTabKeys } from "./side";
import { openAddressInPanel, openInPanel, closePanel } from "./store";
import type { PanelItem } from "./store";
import { patchesOf } from "./pull-request-patches";
import { usePanelReads, useReading } from "./reads";
import type { PatchesRead, PullRequestRead, PullRequestRefused, Reading } from "./reads";

type Item = Extract<PanelItem, { kind: "pr" }>;
type Held = Extract<SessionAttachment, { kind: "pull_request" }>;

const TABS = ["conversation", "checks", "files", "commits", "review"];

const PATCHES_NOT_READ = "The forge did not answer for this pull request's files.";

/** The row the review store takes a pull request in as, for one the lists do not hold. */
function rowOf(read: PullRequestRead, repository: string): PullReviewRow {
  const { facts, page } = read;
  return {
    manifest_id: page.manifest_id,
    repository,
    number: facts.number,
    title: facts.title,
    branch: facts.branch,
    address: facts.address,
    draft: facts.state === "draft",
    checks: facts.checks.state === "failed" ? "failed" : facts.checks.state === "passed" ? "passed" : "running",
    review: page.review,
    head: page.head ?? "",
    author: page.author ?? "",
    updated_at: new Date().toISOString(),
  };
}

type Browser = { browserSaid: string | undefined; openBrowser: (address: string) => void };

/** Open in browser always leaves Armada, and a refusal is said in the panel rather than left a dead press. */
function useBrowser(): Browser {
  const [browserSaid, setSaid] = useState<string | undefined>(undefined);
  const openBrowser = (address: string) => {
    setSaid(undefined);
    void openLink(address).then((followed) => setSaid(whyNotOpenedLink(followed) ?? undefined));
  };
  return { browserSaid, openBrowser };
}

// The panel is drawn the moment it is pressed: the read is two forge round trips, and one that answers nothing must still say so.
export function PullRequestSide({ item }: { item: Item }) {
  const reads = usePanelReads();
  // Read again each time it is pressed (a new `item`) and each time Files is entered: the branch moves under an open panel.
  const [seen, setSeen] = useState(item);
  const [again, setAgain] = useState(0);
  if (seen !== item) {
    setSeen(item);
    setAgain((n) => n + 1);
  }
  const reading = useReading<PullRequestRead | PullRequestRefused>(() => reads?.pullRequest(item) ?? Promise.resolve(undefined), `${item.address}#${item.number}`, again, true);
  if (reads === undefined) return null;
  return <Drawn item={item} reading={reading} onFiles={() => setAgain((n) => n + 1)} />;
}

// One component whether the read is out or in, so the panel's Sheet stays mounted when it lands.
function Drawn({ item, reading, onFiles }: { item: Item; reading: Reading<PullRequestRead | PullRequestRefused>; onFiles: () => void }) {
  const reads = usePanelReads()!;
  const floor = useAtFloor();
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const refused = useSessionsSaid();
  const { browserSaid, openBrowser } = useBrowser();
  const [tab, setTab] = useState(item.tab ?? "conversation");
  const [written, setPage] = useState<PullRequestPage | undefined>(undefined);
  const [kind, setKind] = useState<PullRequestComposerProps["kind"]>("comment");
  const [text, setText] = useState("");
  const [writing, setWriting] = useState(false);
  const [said, setSaid] = useState<string | undefined>(undefined);
  const [pressed, setPressed] = useState<PullRequestAct | undefined>(undefined);
  const here = useRef(true);
  useEffect(() => {
    here.current = true;
    return () => {
      here.current = false;
    };
  }, []);
  const goTo = (next: string) => {
    if (next === "files" && tab !== "files") onFiles();
    setTab(next);
  };
  useTabKeys(TABS, tab, goTo);

  // The Job that holds it, as its ledger does: its attachment is the pull request's live state while it does.
  const owner = sessions.find((one) => one.attachments.some((a) => a.kind === "pull_request" && a.number === item.number));
  const held: Held | undefined = owner?.attachments.find((a): a is Held => a.kind === "pull_request" && a.number === item.number);
  const { sentBack, sendBack } = useSendBack(owner?.id ?? "", draft?.send);
  const back = useBack();

  const answer = reading.out ? undefined : reading.value;
  const refused = answer !== undefined && "refused" in answer ? answer.refused : undefined;
  const read = answer !== undefined && !("refused" in answer) ? answer : undefined;

  // Every file's patch, read the first time Files is shown and kept with the head it was read on: another head is another diff.
  const [patches, setPatches] = useState<{ head: string | undefined; answer: PatchesRead | "reading" } | undefined>(undefined);
  const readPatches = reads.pullRequestPatches;
  const paged = read !== undefined;
  const head = read?.page.head;
  const patchesRun = useRef(0);
  useEffect(() => {
    if (tab !== "files" || !paged || readPatches === undefined || (patches !== undefined && patches.head === head)) return;
    const run = ++patchesRun.current;
    // Dropped, not kept: what is on screen is a diff the branch no longer has.
    setPatches({ head, answer: "reading" });
    void readPatches(item)
      .catch((): PatchesRead => ({ ok: false, said: PATCHES_NOT_READ }))
      .then((answer) => here.current && run === patchesRun.current && setPatches({ head, answer }));
  }, [tab, paged, head, patches, readPatches, item]);

  // The tests that reach each changed file belong to the same diff, so they are read when its patches are. Nothing back, or no read at all, draws no marks.
  const [reach, setReach] = useState<{ head: string | undefined; answer: FilesReach | "reading" } | undefined>(undefined);
  const readReach = reads.testsReaching;
  const reachRun = useRef(0);
  useEffect(() => {
    if (tab !== "files" || !paged || readReach === undefined || (reach !== undefined && reach.head === head)) return;
    const run = ++reachRun.current;
    setReach({ head, answer: "reading" });
    void readReach(item)
      .catch((): undefined => undefined)
      .then((answer) => here.current && run === reachRun.current && setReach({ head, answer: answer ?? {} }));
  }, [tab, paged, head, reach, readReach, item]);
  const shownPatches = useMemo(() => (readPatches === undefined ? undefined : patchesOf(patches === undefined || patches.answer === "reading" ? undefined : patches.answer)), [readPatches, patches]);
  const marks = reach === undefined || reach.answer === "reading" ? undefined : reach.answer;

  if (read === undefined) {
    const repository = item.repository ?? repositoryOfAddress(item.address);
    return (
      <PrOwnersOnTheWire lines={reads.lines}>
        <PullRequestPanel
          reading={reading.out ? {} : { said: refused ?? "The forge did not answer for this pull request." }}
          number={item.number}
          address={item.address}
          {...(repository === undefined ? {} : { repository })}
          {...(item.title === undefined ? {} : { title: item.title })}
          {...(item.branch === undefined ? {} : { branch: item.branch })}
          floor={floor}
          {...(back === undefined ? {} : { back })}
          onOpenBrowser={openBrowser}
          {...(browserSaid === undefined ? {} : { browserSaid })}
          onClose={closePanel}
        />
      </PrOwnersOnTheWire>
    );
  }
  const page = written ?? read.page;
  const facts = held ?? read.facts;
  const keep = `pull-request:panel:${read.repository}#${facts.number}`;

  const act = (what: PullRequestAct) => {
    setPressed(what);
    const done = owner !== undefined && draft !== undefined ? draft.act(owner.id, facts.number, what) : (reads.actOnPullRequest?.(item, what) ?? Promise.resolve(false));
    // A merge asked for here settles the Job's own item asking for it, and that tells the agent.
    const asked = (owner?.waitingFor ?? []).find((one) => one.source === "agent" && one.act?.kind === "approve_pr" && one.act.target.replace(/^#/, "") === String(facts.number));
    if (owner !== undefined && asked !== undefined && (what === "merge" || what === "auto_merge"))
      void done.then((accepted) => {
        if (accepted) draft?.answerWaiting?.(owner.id, asked.id, { text: what === "merge" ? `Merged #${facts.number}` : `Auto-merge is on for #${facts.number}` });
      });
    return done.finally(() => here.current && setPressed(undefined));
  };

  const send = () => {
    if (reads.writePullRequest === undefined || writing || !mayWrite(kind, text)) return;
    const body = text.trim();
    setWriting(true);
    setSaid(undefined);
    void reads.writePullRequest(item, { kind, ...(body === "" ? {} : { body }) }).then((answer) => {
      setWriting(false);
      if (!answer.ok) return setSaid(answer.said);
      forgetKept(keep);
      setPage(answer.page);
      setText("");
      setKind("comment");
    });
  };
  const composer: PullRequestComposerProps | undefined =
    reads.writePullRequest === undefined ? undefined : { kind, onKind: setKind, value: text, onChange: setText, keep, pending: writing, ...(said === undefined ? {} : { said }), onSend: send };

  // A Check's row opens the Check in this same panel; another pull request opens as this one did; the rest is the browser's.
  const onOpenLink = (address: string) => {
    const row = page.checks.find((one) => one.address === address);
    if (row !== undefined) {
      const state = row.state === "skipped" ? "passed" : row.state;
      return void openInPanel({ kind: "check", id: address, name: row.name, branch: facts.branch, row: { state, ...(row.seconds === undefined ? {} : { seconds: row.seconds }), address } });
    }
    if (!openAddressInPanel(address)) void openLink(address);
  };

  const failing = failingOf(facts);
  const description = promptsOut(facts.body ?? page.body);
  const place = placeOf(reads.lines, facts);
  // Named as the forge names it, which is how a registered owner and the Reviews lists key it.
  const forge = item.repository ?? repositoryOfAddress(facts.address) ?? read.repository;
  return (
    <PrOwnersOnTheWire lines={reads.lines} panel={{ repository: forge, number: facts.number, owner: page.owner }}>
      <PullRequestPanel
        number={facts.number}
        title={facts.title}
        branch={facts.branch}
        address={facts.address}
        repository={forge}
        {...(description === "" ? {} : { body: description })}
        page={page}
        {...(composer === undefined ? {} : { composer })}
        state={facts.state}
        checks={facts.checks.state === "failed" || (facts.checks.state === "passed" && facts.checks.red !== undefined) ? { state: "failed", failing } : { state: facts.checks.state }}
        auto={facts.auto}
        queued={facts.queued === true}
        {...(place === undefined ? {} : { place })}
        floor={floor}
        {...(pressed === undefined ? {} : { pending: pressed })}
        refreshing={!reading.out && reading.refreshing}
        onAct={act}
        {...(owner === undefined ? {} : { onSendBack: () => sendBack(facts), sentBack: sentBack(facts) })}
        onOpenLink={onOpenLink}
        onOpenBrowser={openBrowser}
        {...(browserSaid === undefined ? {} : { browserSaid })}
        onClose={closePanel}
        {...(back === undefined ? {} : { back })}
        tab={tab}
        onTab={goTo}
        {...(shownPatches === undefined ? {} : { patches: shownPatches })}
        {...(marks === undefined ? {} : { reach: marks, onTest: (test: ReachTest) => void openInPanel({ kind: "file", path: read.root === undefined ? test.test : `${read.root}/${test.test}`, from: "session" }) })}
        more={{ id: "review", label: "Review", body: <ReviewTab row={rowOf(read, forge)} onOpenLink={onOpenLink} /> }}
      >
        {/* A press the Job made and Fleet refused is said here, where it was pressed; one nobody holds is a toast. */}
        {owner === undefined || refused === undefined || refused === "" ? null : <Alert tone="escalated">{refused}</Alert>}
      </PullRequestPanel>
    </PrOwnersOnTheWire>
  );
}
