import { useEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { Check, Clock, Copy, ExternalLink, FileText, GitMerge, GitPullRequest } from "lucide-react";
import type { PullRequestPage } from "@armada/protocol";

import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { Skeleton, SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { Sheet, type SheetBack } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { PlaceRow } from "../MergeLine/MergeLine";
import { PullRequestActs, type PullRequestActsProps } from "../PullRequestActs/PullRequestActs";
import { RegisteredOwnerOf } from "../RegisteredOwner/RegisteredOwner";
import { PullRequestFix, type PullRequestFixProps } from "./PullRequestFix";
import { PullRequestComposer, type PullRequestComposerProps } from "./PullRequestComposer";
import { PullRequestPlates, type PullRequestChecks } from "./PullRequestPlates";
import { PatchStack, type PullRequestPatches } from "./PullRequestFiles";
import { DiffSize, FileRows, PullRequestTabs, type PullRequestTabsProps } from "./PullRequestTabs";
import type { FilesReach, ReachTest } from "./PullRequestReach";
import { agoOf } from "./pull-request-format";

export type { PullRequestPatch, PullRequestPatches } from "./PullRequestFiles";
export type { PullRequestFixProps, PullRequestFixRun } from "./PullRequestFix";

/**
 * A pull request as a panel docked to the trailing edge, with its status, its merge queue and its
 * failing checks as plates floating off its leading edge. The head is Sheet's: the pull request's
 * number names the dialog, its repository sits under it, and Close ends it. Below the head, the
 * title, the state, the branch and the acts, with Open in browser at the row's trailing end.
 *
 * **Drawn only from what Fleet serves** (`PullRequestState`, the merge line, and the page read
 * when the panel opens): the tabs, the comment box, the author, the diff size, the base branch, the
 * reviews and the conflicts come from the page, and an absent fact leaves its slot empty. With no
 * page the description is the last thing in the body, drawn as Markdown; with one it is the first
 * entry of the Conversation tab. At the window floor the plates are the panel's own first rows,
 * since nothing is left beside it.
 *
 * **With `patches` the Files tab is every file's diff, one under the other**, and the file list
 * floats in a sheet to the panel's left while Files is the tab: a press scrolls the body to that
 * file and marks it. At the floor, where that sheet would lie over the panel, the list heads the
 * tab instead.
 */
export type PullRequestPanelProps = {
  number: number;
  title: string;
  branch: string;
  /** Where it is on the forge. Open in browser takes it. */
  address: string;
  /** `owner/name`, where the address names it. */
  repository?: string;
  state: "draft" | "open" | "merged";
  checks: PullRequestChecks;
  auto: boolean;
  queued?: boolean;
  /** The merge line it is in, from `placeOf`. Absent draws no queue plate. */
  place?: readonly PlaceRow[];
  /** The window is at `--window-floor`: the plates move into the panel. */
  floor?: boolean;
  /** The act pressed that Fleet has not answered yet. */
  pending?: PullRequestActsProps["pending"];
  onAct: PullRequestActsProps["onAct"];
  /**
   * Tells the Session the panel was opened from that its checks failed. Absent where no Session
   * is the owner of the pull request, which draws no such act.
   */
  onSendBack?: () => void;
  /** The Session has been told, for these failing checks. */
  sentBack?: boolean;
  onOpenLink: (address: string) => void;
  /** Open in browser, which always leaves Armada. Absent sends the address to `onOpenLink`. */
  onOpenBrowser?: (address: string) => void;
  /** Why Open in browser did not open, in the host's words. */
  browserSaid?: string;
  onClose: () => void;
  /** What its author wrote of it, as Markdown. Absent or blank draws nothing. */
  body?: string;
  /** What Fleet served of the pull request beyond its state. Absent draws no tabs, no comment box and none of its facts. */
  page?: PullRequestPage;
  /** Why the page was not read, in Fleet's words. The panel stays as it is without the tabs. */
  pageSaid?: string;
  /** The comment box. Absent draws none. */
  composer?: PullRequestComposerProps;
  /** Every file's diff for the Files tab, as the host read them. Absent leaves the Files tab a list, with nothing beside the panel. */
  patches?: PullRequestPatches;
  /** The tests that reach each changed file, from the test map. Absent draws none, and a path with no entry draws no mark. */
  reach?: FilesReach;
  /** A test opened from under its file, where it names the file it lives in. */
  onTest?: (test: ReachTest) => void;
  /** A tab of the host's own after the four, and the tab shown where the host holds it. Drawn with the page's tabs only. */
  more?: PullRequestTabsProps["more"];
  /** What is drawn is a kept answer and the read goes out once more: a thin bar says so until it lands. */
  refreshing?: boolean;
  /** The way back to what the panel was opened from, where it was opened from another panel. */
  back?: SheetBack;
  tab?: string;
  onTab?: (id: string) => void;
  /** What Fleet refused, or anything else the host holds for the body. */
  children?: ReactNode;
  reading?: undefined;
};

/**
 * The panel the moment a pull request is pressed, before the forge's read answers: the same head,
 * what the press already carried (its title and branch), and stand-ins for the rest. **The same
 * component as the filled panel**, so its Sheet stays mounted when the read lands: it does not
 * travel in a second time or take focus back. With `reading.said` the read came back with nothing,
 * and the stand-ins give way to that sentence.
 */
export type PullRequestPanelReadingProps = {
  reading: { said?: string };
  /** The step the sentence names, drawn under it with a Run and a Copy. Absent where it names none. */
  fix?: PullRequestFixProps;
  number: number;
  /** Where it is on the forge. Empty draws no Open in browser. */
  address: string;
  repository?: string;
  title?: string;
  branch?: string;
  floor?: boolean;
  back?: SheetBack;
  onOpenBrowser: (address: string) => void;
  browserSaid?: string;
  onClose: () => void;
};

const PILL = {
  merged: { status: "completed-success", Icon: GitMerge, said: "Merged" },
  queued: { status: "running", Icon: Clock, said: "In merge queue" },
  draft: { status: "not-started", Icon: GitPullRequest, said: "Draft" },
  open: { status: "completed-success", Icon: GitPullRequest, said: "Open" },
} as const;

type Head = { number: number; repository?: string | undefined; floor: boolean; back?: SheetBack | undefined; onClose: () => void };

/** The Sheet's head, one for the panel waiting on its read and the panel filled, so nothing in it moves when the read lands. */
const headOf = ({ number, repository, floor, back, onClose }: Head) => ({
  kind: "pull-request",
  open: true,
  floating: true,
  floor,
  title: `Pull request #${number}`,
  ...(repository === undefined ? {} : { subtitle: <span className="armada-pr-panel__repository mono">{repository}</span> }),
  size: "panel" as const,
  ...(back === undefined ? {} : { back }),
  closeLabel: "Close",
  closeBinding: "Esc",
  onClose,
});

/** Puts the pull request's address on the clipboard, and says so for a moment. */
function CopyLink({ address }: { address: string }) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);
  if (address === "") return null;
  const said = copied ? "Copied" : "Copy link";
  return (
    <span className="armada-pr-panel__open">
      <Tooltip label={said}>
        <Button variant="ghost" size="sm" iconOnly aria-label={said} onClick={() => void navigator.clipboard.writeText(address).then(() => setCopied(true))}>
          {copied ? <Check size={16} strokeWidth={2} aria-hidden /> : <Copy size={16} strokeWidth={2} aria-hidden />}
        </Button>
      </Tooltip>
    </span>
  );
}

function OpenInBrowser({ onPress }: { onPress: () => void }) {
  return (
    <span className="armada-pr-panel__open">
      <Tooltip label="Open in browser">
        <Button variant="ghost" size="sm" iconOnly aria-label="Open in browser" onClick={onPress}>
          <ExternalLink size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </span>
  );
}

const BrowserSaid = ({ said }: { said: string | undefined }) =>
  said === undefined || said === "" ? null : (
    <p className="armada-pr-panel__said" role="alert">
      {said}
    </p>
  );

function Reading({ reading, fix, number, address, title, branch, onOpenBrowser, browserSaid }: PullRequestPanelReadingProps) {
  const out = reading.said === undefined;
  const named = title !== undefined && title !== "";
  const branched = branch !== undefined && branch !== "";
  return (
    <div className="armada-pr-panel">
      {named ? (
        <h3 className="armada-pr-panel__title">{title}</h3>
      ) : out ? (
        <div className="armada-pr-panel__title armada-pr-panel__stand-in">
          <Skeleton width="70%" />
        </div>
      ) : null}
      {branched || out ? (
        <div className="armada-pr-panel__meta armada-pr-panel__stand-in">
          {out ? <Skeleton width="15%" /> : null}
          {branched ? (
            <Tooltip label="Branch">
              <span className="armada-pr-panel__branch mono">{branch}</span>
            </Tooltip>
          ) : (
            <Skeleton width="35%" />
          )}
        </div>
      ) : null}
      <div className="armada-pr-panel__acts armada-pr-panel__stand-in" role="group" aria-label="Acts">
        {out ? <Skeleton width="30%" /> : null}
        <CopyLink address={address} />
        {address === "" ? null : <OpenInBrowser onPress={() => onOpenBrowser(address)} />}
      </div>
      <BrowserSaid said={browserSaid} />
      {out ? (
        <SkeletonText label={`Reading pull request #${number}`} widths={["100%", "85%", "60%", "90%", "40%"]} />
      ) : (
        <p className="armada-pr-panel__said" role="alert">
          {reading.said}
        </p>
      )}
      {fix === undefined || out ? null : <PullRequestFix {...fix} />}
    </div>
  );
}

// Two signatures rather than a union, so `ComponentProps` (and Storybook's args) read the filled panel's, the last.
export function PullRequestPanel(props: PullRequestPanelReadingProps): ReactNode;
export function PullRequestPanel(props: PullRequestPanelProps): ReactNode;
export function PullRequestPanel(props: PullRequestPanelProps | PullRequestPanelReadingProps): ReactNode {
  const head = headOf({ number: props.number, repository: props.repository, floor: props.floor ?? false, back: props.back, onClose: props.onClose });
  // Held above both returns, so the hooks are the same whether the read is out or in.
  const [ownTab, setOwnTab] = useState("conversation");
  const [current, setCurrent] = useState<string | undefined>(undefined);
  const sections = useRef(new Map<string, HTMLElement>());
  const body = useRef<HTMLDivElement>(null);
  const filledProps = props.reading === undefined ? props : undefined;
  const tab = filledProps?.tab ?? ownTab;
  const listed = (filledProps?.page?.files.length ?? 0) > 0;
  const floats = filledProps?.patches !== undefined && tab === "files" && !(filledProps.floor ?? false) && listed;
  // The list follows the tab rather than being opened, so the tab pressed keeps focus rather than the list's close taking it.
  useEffect(() => {
    if (floats) body.current?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')?.focus({ preventScroll: true });
  }, [floats]);
  // Both returns open on the one Sheet, so React keeps it mounted from one to the other.
  if (props.reading !== undefined)
    return (
      <>
        <Sheet {...head}>
          <Reading {...props} />
        </Sheet>
        {null}
      </>
    );
  // Called, not mounted: a component of its own would put another type above the Sheet.
  return filled(props, {
    tab,
    onTab: props.onTab ?? setOwnTab,
    floats,
    current,
    body,
    target: (path) => (element) => void (element === null ? sections.current.delete(path) : sections.current.set(path, element)),
    onFile: (path) => {
      setCurrent(path);
      sections.current.get(path)?.scrollIntoView({ block: "start" });
    },
  });
}

/** What the panel holds of its own for the Files tab: which tab is shown, whether the list floats, and where a file's diff is. */
type Own = {
  tab: string;
  onTab: (id: string) => void;
  floats: boolean;
  current: string | undefined;
  body: RefObject<HTMLDivElement | null>;
  target: (path: string) => (element: HTMLElement | null) => void;
  onFile: (path: string) => void;
};

function filled({
  number,
  title,
  branch,
  address,
  repository,
  state,
  checks,
  auto,
  queued = false,
  place,
  floor = false,
  pending,
  refreshing = false,
  onAct,
  onSendBack,
  sentBack = false,
  onOpenLink,
  onOpenBrowser,
  browserSaid,
  onClose,
  body,
  page,
  pageSaid,
  composer,
  patches,
  reach,
  onTest,
  more,
  back,
  children,
}: PullRequestPanelProps, { tab, onTab, floats, current, body: bodyRef, target, onFile }: Own) {
  const pill = PILL[state === "merged" ? "merged" : queued ? "queued" : state];
  const plates = <PullRequestPlates state={state} checks={checks} auto={auto} queued={queued} {...(page === undefined ? {} : { review: page.review, approvals: page.approvals, mergeable: page.mergeable, ...(page.reviewers === undefined ? {} : { reviewers: page.reviewers }) })} {...(place === undefined ? {} : { place })} onOpenPullRequest={onOpenLink} {...(onSendBack === undefined ? {} : { onSendBack })} sentBack={sentBack} />;
  const description = body?.trim() ?? "";
  const opened = agoOf(page?.created_at);
  // The file list lies where the plates would, so they are not drawn while it is open; at the floor they move into the body.
  const platesInBody = floor;
  const tone = state === "merged" ? "success" : queued ? "running" : checks.state === "failed" ? "failed" : checks.state === "passed" ? "success" : "neutral";
  return (
    <>
    <Sheet
      {...headOf({ number, repository, floor, back, onClose })}
      under={floats}
      {...(platesInBody || floats ? {} : { aside: <div className="armada-pr-plates">{plates}</div> })}
      {...(composer === undefined ? {} : { footer: <PullRequestComposer {...composer} /> })}
    >
      <div className="armada-pr-panel" ref={bodyRef}>
        {refreshing ? <div role="progressbar" aria-label={`Refreshing pull request #${number}`} className="armada-pr-panel__refresh" /> : null}
        <h3 className="armada-pr-panel__title">{title}</h3>
        <div className="armada-pr-panel__meta">
          <Badge status={pill.status} icon={pill.Icon}>
            {pill.said}
          </Badge>
          <Tooltip label={page === undefined ? "Branch" : "Branch and the branch it merges into"}>
            <span className="armada-pr-panel__branch mono">{page === undefined ? branch : `${branch} \u2192 ${page.base}`}</span>
          </Tooltip>
          {page?.author === undefined ? null : (
            <Tooltip label="Opened by">
              <span className="armada-pr-panel__who">{page.author}</span>
            </Tooltip>
          )}
          {opened === undefined ? null : (
            <Tooltip label="Opened">
              <span className="armada-pr-panel__who">{opened}</span>
            </Tooltip>
          )}
          {page === undefined ? null : <DiffSize additions={page.additions} deletions={page.deletions} />}
          <RegisteredOwnerOf repository={repository} number={number} />
        </div>
        <div className="armada-pr-panel__acts" role="group" aria-label="Acts">
          <PullRequestActs state={state} checks={checks.state} auto={auto} queued={queued} {...(pending === undefined ? {} : { pending })} onAct={onAct} />
          <CopyLink address={address} />
          <OpenInBrowser onPress={() => (onOpenBrowser ?? onOpenLink)(address)} />
        </div>
        <BrowserSaid said={browserSaid} />
        {platesInBody ? <div className="armada-pr-plates">{plates}</div> : null}
        {children}
        {pageSaid === undefined || pageSaid === "" ? null : (
          <p className="armada-pr-panel__said" role="alert">
            {pageSaid}
          </p>
        )}
        {page === undefined ? null : (
          <PullRequestTabs
            page={page}
            description={description}
            onOpenLink={onOpenLink}
            {...(patches === undefined
              ? {}
              : {
                  files: (
                    <>
                      {floor ? <FileRows rows={page.files} current={current} onFile={onFile} reach={reach} onTest={onTest} /> : null}
                      <PatchStack rows={page.files} patches={patches} target={target} />
                    </>
                  ),
                })}
            reach={reach}
            onTest={onTest}
            {...(more === undefined ? {} : { more })}
            tab={tab}
            onTab={onTab}
          />
        )}
        {page !== undefined || description === "" ? null : (
          <section className="armada-pr-plate armada-pr-panel__description" data-tone={tone} aria-label="Description">
            <Tooltip label="Description">
              <span className="armada-pr-plate__eyebrow" role="img" aria-label="Description">
                <FileText size={16} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <Prose text={description} />
          </section>
        )}
      </div>
    </Sheet>
    {floats && page !== undefined ? (
      /* Beside the panel, as Plan's task panel holds a file: a second floating sheet to its left that
         dims nothing. It is the panel's own, so its close and Esc close the panel. */
      <Sheet open floating beside="pull-request" kind="pull-request-files" size="panel" title="Files" closeBinding="Esc" onClose={onClose}>
        <FileRows rows={page.files} current={current} onFile={onFile} reach={reach} onTest={onTest} />
      </Sheet>
    ) : null}
    </>
  );
}
