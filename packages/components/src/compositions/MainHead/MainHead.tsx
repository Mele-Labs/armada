import { Fragment, useState, type ReactNode } from "react";
import { Clock, GitMerge, Hammer, Monitor, SquareTerminal } from "lucide-react";

import { CHECK_OUTCOME } from "../../generated/vocabulary";
import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { since } from "../../since";
import { RegisteredOwner, type RegisteredOwnerRow } from "../RegisteredOwner/RegisteredOwner";

/**
 * Main's state at the head of the merge line, as the hub draws it. Fleet serves it as the line's
 * `hub` since protocol 23.41, and `packages/screens/src/merge-line.ts` folds it onto these types.
 *
 * Green is an icon and its tooltip beside the heading. Red is a band under it that stays
 * where the panel is folded: the failing Check, the failing test and the merge that turned it
 * red, each a link, then either the Session that took it or the ways to hand it to one.
 */

/** A Session, by its id and what it is called. */
export type HubSession = { id: string; title: string };

/** A Session the work can be sent back to, as a message. */
export type RecentSession = HubSession & { branch?: string };

/** What turned main red. */
export type MainRed = {
  /**
   * What failed on main, as the forge reports it: a Manifest Check's name where the failing CI job
   * maps to one, otherwise the CI job's own name, and `unmapped` says which.
   */
  check: string;
  unmapped?: true;
  /** The failing test, where Armada could read one out of the log. Absent draws no row. */
  test?: string;
  /** Where the test is, at the merge. Absent draws the test as text. */
  testUrl?: string;
  /** Any other CI job that failed, each as the first is. Absent is the one. */
  also?: readonly { check: string; unmapped?: true; test?: string }[];
  /** When main first read red, RFC 3339, kept while it stays red. Absent from a Fleet before it carried it. */
  since?: string;
  /** The pull request that merged. Absent where the forge named none, as a direct push does not. */
  merge?: {
    number: number;
    /** Absent where only the merge commit's message named the pull request: its number is text. */
    url?: string;
    /** Absent where the forge did not say. */
    branch?: string;
    /** Absent draws the branch as text. */
    branchUrl?: string;
    /** The Session whose pull request it was. Absent for a person's. */
    session?: HubSession;
  };
};

/**
 * Another machine's claim on main's red, from the central store: who holds it, where, and since when
 * (RFC 3339). No title, since only the holder can say what it is called.
 */
export type HeldElsewhere = {
  machine: string;
  holder: { kind: "session"; id: string };
  since: string;
};

/** A claim older than this reads as stale. */
const STALE_AFTER_MS = 60 * 60_000;

/** A newer commit on main whose CI is still running: its pull request where the forge named one, else the commit. */
export type MainChecking = { commit: string; number?: number; url?: string };

export type MainState =
  /** `checking` non-empty is only checks running on main: the mark says so, and no band is drawn. */
  | { state: "green"; checking?: readonly MainChecking[] }
  /**
   * `takenBySession` is the Session working on it: absent until one is. **`checking` non-empty is a held red**: a newer run may already have fixed
   * it, so the band is caution, says so, and offers no way to hand it to a Session. Absent or empty is a
   * red to act on.
   */
  | { state: "red"; red: MainRed; takenBySession?: HubSession; elsewhere?: HeldElsewhere; checking?: readonly MainChecking[] };

/**
 * What the owner chose: a Session to send it to as a message, or a new Session, which the Dispatch panel
 * starts from this brief. Taking over a claim another machine holds is a new Session as well.
 */
export type FixChoice =
  | { kind: "session"; session: string; message: string }
  | { kind: "newSession"; request: string }
  /** Replace the claim another machine holds: the Dispatch panel starts a new Session from this brief. */
  | { kind: "takeOver"; request: string };

/**
 * What Fleet said to a press, where the band draws it: the store that holds claims could not be
 * reached, or another machine's claim was not yet in the hub when the press was made.
 */
export type FixRefusal = "claim_not_made" | "taken_elsewhere";

/** What the band says of each refusal. */
const REFUSED: Record<FixRefusal, string> = {
  claim_not_made: "Not made, store unreachable",
  taken_elsewhere: "Held on another machine",
};

const MARK = 16;
const SMALL = 12;
const STROKE = 2;

/** Main's own state beside the heading. Green says nothing but the tooltip. */
export function MainMark({ main }: { main: MainState }) {
  const reading = CHECK_OUTCOME[main.state === "green" ? "passed" : "failed"];
  const Icon = reading?.icon ?? null;
  const checking = (main.checking?.length ?? 0) > 0;
  const held = main.state === "red" && checking;
  const said =
    main.state === "green"
      ? checking
        ? "Main is green, new checks are running"
        : "Main is green"
      : held
        ? "Main is red, new checks are running"
        : "Main is red";
  if (Icon === null) return null;
  return (
    <Tooltip label={said} asChild>
      <span
        className="armada-main-mark"
        data-checking={(main.state === "green" && checking) || undefined}
        role="img"
        aria-label={said}
        style={held ? { color: "var(--notice-caution)" } : reading?.statusToken ? { color: `var(${reading.statusToken})` } : undefined}
      >
        <Icon size={MARK} strokeWidth={STROKE} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * A Job that took main's red, or fixed it, as a mark beside its status badge. **An icon and its
 * tooltip, no phrase.** The hammer pulses while it works and holds still under reduced motion.
 */
export function FixingMainMark({ state, said }: { state: "fixing" | "fixed"; said: string }) {
  const Fixed = CHECK_OUTCOME.passed?.icon ?? null;
  return (
    <Tooltip label={said}>
      <span className="armada-fixing-mark" data-state={state} role="img" aria-label={said}>
        {state === "fixing" ? (
          <Hammer size={SMALL} strokeWidth={STROKE} aria-hidden />
        ) : Fixed === null ? null : (
          <Fixed size={SMALL} strokeWidth={STROKE} aria-hidden />
        )}
      </span>
    </Tooltip>
  );
}

export type MainRedBandProps = {
  main: Extract<MainState, { state: "red" }>;
  /** Earlier Sessions, newest first. The culprit's own is drawn first wherever it is among them. Absent is none. */
  recentSessions?: readonly RecentSession[];
  onOpenLink: (url: string) => void;
  /** Opens the failed Check's log. Absent, the Check is not a press. */
  onOpenCheck?: (check: string) => void;
  /** Opens the Session that merged. Absent, its title is not a press. */
  onOpenSession?: (sessionId: string) => void;
  /**
   * Told the owner's choice. **The ways to hand the red to a Session draw only where something
   * listens**: a Fleet that cannot take one offers none, and a button that does nothing is worse.
   */
  onFix?: (choice: FixChoice) => void | Promise<FixRefusal | undefined | void>;
  /** The moment ages are counted from; the clock, unless a story fixes it. */
  now?: number;
};

export function MainRedBand({ main, recentSessions = [], onOpenLink, onOpenCheck, onOpenSession, onFix, now }: MainRedBandProps) {
  const { red, takenBySession, elsewhere } = main;
  // A Session holds it here: the acts are moot, and the Fixing row names it.
  const heldHere = takenBySession !== undefined;
  const checking = main.checking ?? [];
  const held = checking.length > 0;
  // The acts draw only where something listens, and not where a held red or a Session on it makes them moot.
  const free = !held && !heldHere && onFix !== undefined;
  // Another machine holds the red: its Session is not ours to start, and the one act left is to take it over.
  const asks = free && elsewhere === undefined;
  const takesOver = free && elsewhere !== undefined;
  const [asking, setAsking] = useState<"back" | "takeOver" | null>(null);
  // The press Fleet refused, kept so Try again sends the same one.
  const [refused, setRefused] = useState<{ choice: FixChoice; why: FixRefusal } | null>(null);
  const send = async (choice: FixChoice) => {
    setRefused(null);
    const why = await onFix?.(choice);
    if (why === "claim_not_made" || why === "taken_elsewhere") setRefused({ choice, why });
  };
  const claim =
    elsewhere === undefined || heldHere ? null : (
      <HeldBy held={elsewhere} {...(now === undefined ? {} : { now })}>
        {!takesOver ? null : (
          <Button variant="ghost" size="sm" ground="sunken" onClick={() => setAsking("takeOver")}>
            Take over
          </Button>
        )}
      </HeldBy>
    );
  const link = (url: string) => (event: { preventDefault: () => void }) => {
    event.preventDefault();
    onOpenLink(url);
  };
  return (
    <div className="armada-main-red" data-held={held || undefined}>
      <div className="armada-main-red__frame" role="status" aria-label={held ? "New checks are running on main" : "Main is red"}>
        <div className="armada-main-red__head">
          {held ? (
            <span className="armada-main-red__title">
              New checks are running on main:{" "}
              {checking.map((one, at) => (
                <Fragment key={one.commit}>
                  {at === 0 ? null : " "}
                  {one.number === undefined ? (
                    <span className="mono">{one.commit.slice(0, 10)}</span>
                  ) : one.url === undefined ? (
                    <span>#{one.number}</span>
                  ) : (
                    <a className="armada-main-red__link mono" href={one.url} onClick={link(one.url)}>
                      #{one.number}
                    </a>
                  )}
                </Fragment>
              ))}
            </span>
          ) : (
            <span className="armada-main-red__title">Main is red</span>
          )}
          {!asks ? null : (
            <span className="armada-main-red__ask">
              <Button variant="secondary" size="sm" ground="sunken" onClick={() => void send({ kind: "newSession", request: briefOf(red) })}>
                Dispatch a new Job
              </Button>
              {red.merge !== undefined ? null : (
                <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("back")}>
                  Send back to…
                </Button>
              )}
            </span>
          )}
        </div>
        <dl className="armada-main-red__rows mono">
          {[{ check: red.check, unmapped: red.unmapped, test: red.test, testUrl: red.testUrl }, ...(red.also ?? [])].map(
            (job, at) => (
              <Fragment key={`${job.check}:${at}`}>
                <dt>{job.unmapped === true ? "CI job" : "Check"}</dt>
                <dd>
                  {onOpenCheck === undefined ? (
                    job.check
                  ) : (
                    <button type="button" className="armada-main-red__link mono" onClick={() => onOpenCheck(job.check)}>
                      {job.check}
                    </button>
                  )}
                </dd>
                {job.test === undefined ? null : (
                  <>
                    <dt>Test</dt>
                    <dd>
                      {"testUrl" in job && job.testUrl !== undefined ? (
                        <a className="armada-main-red__link mono" href={job.testUrl} onClick={link(job.testUrl)}>
                          {job.test}
                        </a>
                      ) : (
                        job.test
                      )}
                    </dd>
                  </>
                )}
              </Fragment>
            ),
          )}
          {red.merge === undefined ? null : (
            <>
              <dt>Broke in</dt>
              <dd className="armada-main-red__merge">
                {red.merge.url === undefined ? (
                  <span>#{red.merge.number}</span>
                ) : (
                  <a className="armada-main-red__link mono" href={red.merge.url} onClick={link(red.merge.url)}>
                    #{red.merge.number}
                  </a>
                )}
                {red.merge.branch === undefined ? null : red.merge.branchUrl === undefined ? (
                  <span>{red.merge.branch}</span>
                ) : (
                  <a className="armada-main-red__link mono" href={red.merge.branchUrl} onClick={link(red.merge.branchUrl)}>
                    {red.merge.branch}
                  </a>
                )}
                <MergeOwner merge={red.merge} {...(onOpenSession === undefined ? {} : { onOpenSession })} />
                {!asks ? null : (
                  <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("back")}>
                    {red.merge.session !== undefined ? "Send back" : "Send back to…"}
                  </Button>
                )}
              </dd>
            </>
          )}
          {refused === null || heldHere || elsewhere !== undefined ? null : (
            <>
              <dt>Claim</dt>
              <dd className="armada-main-red__taken">
                <span>{REFUSED[refused.why]}</span>
                <Button variant="ghost" size="sm" ground="sunken" onClick={() => void send(refused.choice)}>
                  Try again
                </Button>
              </dd>
            </>
          )}
          {claim === null ? null : (
            <>
              <dt>Fixing</dt>
              <dd className="armada-main-red__taken">{claim}</dd>
            </>
          )}
          {takenBySession === undefined ? null : (
            <>
              <dt>Fixing</dt>
              <dd className="armada-main-red__taken">
                <Tooltip label="Working on it" asChild>
                  <span className="armada-fixing-mark" data-state="fixing" role="img" aria-label="Working on it">
                    <Hammer size={SMALL} strokeWidth={STROKE} aria-hidden />
                  </span>
                </Tooltip>
                <OwnerLink kind="Job" Icon={SquareTerminal} one={takenBySession} open={onOpenSession} />
              </dd>
            </>
          )}
        </dl>
      </div>
      {asking !== "takeOver" || elsewhere === undefined ? null : (
        <TakeOverDialog
          held={elsewhere}
          {...(now === undefined ? {} : { now })}
          onCancel={() => setAsking(null)}
          onConfirm={() => {
            setAsking(null);
            void send({ kind: "takeOver", request: briefOf(red) });
          }}
        />
      )}
      {asking === null || asking === "takeOver" ? null : (
        <FixDialog
          red={red}
          recentSessions={recentSessions}
          onCancel={() => setAsking(null)}
          onFix={(choice) => {
            setAsking(null);
            void send(choice);
          }}
        />
      )}
    </div>
  );
}

/** The holder's registry chip, as a pull request's owner is drawn: kind and short id, then `monitor` and the machine's name. */
const chipOf = (held: HeldElsewhere): RegisteredOwnerRow => ({ kind: held.holder.kind, id: held.holder.id, machine: { name: held.machine, here: false } });

/** Another machine's claim: who holds it, and for how long. A claim long held reads as stale by its mark and age alone. */
function HeldBy({ held, now = Date.now(), children }: { held: HeldElsewhere; now?: number; children?: ReactNode }) {
  const age = since(held.since, now);
  const stale = now - Date.parse(held.since) >= STALE_AFTER_MS;
  return (
    <span className="armada-main-red__held" data-stale={stale || undefined}>
      <RegisteredOwner row={chipOf(held)} />
      {age === undefined ? null : (
        <Tooltip label={stale ? `Claimed ${age} ago, stale` : `Claimed ${age} ago`} asChild>
          <span className="armada-main-red__age" role="img" aria-label={stale ? `Claimed ${age} ago, stale` : `Claimed ${age} ago`}>
            {!stale ? null : <Clock size={SMALL} strokeWidth={STROKE} aria-hidden />}
            <span className="mono">{age}</span>
          </span>
        </Tooltip>
      )}
      {children}
    </span>
  );
}

/** Asked once, with the claim's age in it, before another machine's claim is replaced. */
function TakeOverDialog({ held, now, onCancel, onConfirm }: { held: HeldElsewhere; now?: number; onCancel: () => void; onConfirm: () => void }) {
  return (
    <Dialog
      open
      tone="neutral"
      title={`Take over from ${held.machine}`}
      titleNode={
        <>
          Take over from{" "}
          <span className="armada-main-red__machine">
            <Monitor size={SMALL} strokeWidth={STROKE} aria-hidden />
            {held.machine}
          </span>
        </>
      }
      confirmLabel="Take over"
      onCancel={onCancel}
      onConfirm={onConfirm}
    >
      <HeldBy held={held} {...(now === undefined ? {} : { now })} />
    </Dialog>
  );
}

/**
 * Who merged: the Session that owns the pull request, by its registry icon and its title, which
 * opens it. Nobody known draws nothing.
 */
function MergeOwner({ merge, onOpenSession }: { merge: NonNullable<MainRed["merge"]>; onOpenSession?: (sessionId: string) => void }) {
  return merge.session === undefined ? null : <OwnerLink kind="Job" Icon={SquareTerminal} one={merge.session} open={onOpenSession} />;
}

/** A Session by its registry icon and its title, which opens it where something listens. */
function OwnerLink({ kind, Icon, one, open }: { kind: string; Icon: typeof SquareTerminal; one: HubSession; open: ((id: string) => void) | undefined }) {
  return (
    <span className="armada-main-red__owner">
      <Tooltip label={kind} asChild>
        <span className="armada-main-red__owner-mark" role="img" aria-label={kind}>
          <Icon size={SMALL} strokeWidth={STROKE} aria-hidden />
        </span>
      </Tooltip>
      {open === undefined ? (
        <span>{one.title}</span>
      ) : (
        <button type="button" className="armada-main-red__link" onClick={() => open(one.id)}>
          {one.title}
        </button>
      )}
    </span>
  );
}

/** What the first row is called: Armada adds Check meaning only where the CI job maps to a Manifest Check. */
const labelOf = (red: MainRed) => (red.unmapped === true ? "CI job" : "Check");

/** What goes with the work wherever it is sent: the Check, its test, its log and the merge. */
function Attached({ red }: { red: MainRed }) {
  return (
    <div className="armada-main-fix__attached" role="list" aria-label="Attached">
      <span role="listitem">
        <AttachmentChip from={labelOf(red)} filename={red.check} />
      </span>
      {red.test === undefined ? null : (
        <span role="listitem">
          <AttachmentChip from="Test" filename={red.test} />
        </span>
      )}
      <span role="listitem">
        <AttachmentChip from="Log" filename={`${red.check}.log`} />
      </span>
      {red.merge === undefined ? null : (
        <span role="listitem">
          <AttachmentChip from="Pull request" filename={[`#${red.merge.number}`, red.merge.branch].filter(Boolean).join(" ")} />
        </span>
      )}
    </div>
  );
}

/** The brief a new Session starts from: bare facts, there to be edited. */
export function briefOf(red: MainRed): string {
  return [
    `${red.check} fails on main.`,
    ...(red.test === undefined ? [] : [`Test: ${red.test}`]),
    ...(red.merge === undefined ? [] : [`Merged in #${red.merge.number}${red.merge.branch === undefined ? "" : ` (${red.merge.branch})`}.`]),
  ].join("\n");
}

/** The culprit's own Session first, then the rest as they came. */
export function culpritFirst<T extends { id: string }>(recent: readonly T[], culprit: { id: string } | undefined): T[] {
  const own = recent.filter((one) => one.id === culprit?.id);
  return [...own, ...recent.filter((one) => one.id !== culprit?.id)];
}

function FixDialog({
  red,
  recentSessions,
  onCancel,
  onFix,
}: {
  red: MainRed;
  recentSessions: readonly RecentSession[];
  onCancel: () => void;
  onFix: (choice: FixChoice) => void;
}) {
  const [request, setRequest] = useState(() => briefOf(red));
  const sessions = culpritFirst(recentSessions, red.merge?.session);
  // The culprit is chosen before the owner looks: the Session that merged it.
  const [picked, setPicked] = useState<string | null>(() =>
    red.merge?.session !== undefined && sessions[0]?.id === red.merge.session.id ? red.merge.session.id : null,
  );
  const mark = (id: string, owner: { id: string } | undefined) =>
    red.merge === undefined || owner === undefined || id !== owner.id ? null : (
      <Tooltip label={`Merged #${red.merge.number}, the pull request that turned main red`} asChild>
        <span className="armada-main-fix__culprit" role="img" aria-label={`Merged #${red.merge.number}`}>
          <GitMerge size={SMALL} strokeWidth={STROKE} aria-hidden />
        </span>
      </Tooltip>
    );
  return (
    <Dialog
      open
      tone="neutral"
      width="wide"
      title="Send the work back"
      confirmLabel="Send back"
      confirmDisabled={picked === null || request.trim() === ""}
      onCancel={onCancel}
      onConfirm={() => {
        if (picked !== null) onFix({ kind: "session", session: picked, message: request });
      }}
    >
      <Attached red={red} />
      {sessions.length === 0 ? null : (
        <RadioGroup label="Jobs">
          {sessions.map((one) => (
            <Radio key={one.id} name="main-fix-session" value={one.id} checked={picked === one.id} onChange={() => setPicked(one.id)}>
              <span className="armada-main-fix__job">
                <span>{one.title}</span>
                {one.branch === undefined ? null : <span className="mono armada-main-fix__branch">{one.branch}</span>}
                {mark(one.id, red.merge?.session)}
              </span>
            </Radio>
          ))}
        </RadioGroup>
      )}
      {picked !== null ? <Textarea label="Message" rows={5} value={request} onChange={(event) => setRequest(event.target.value)} /> : null}
    </Dialog>
  );
}
