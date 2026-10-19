// What fills the flight deck's stage once something is picked: a Session as the Sessions page's own panel, a bay as the facts about
// it and who held it, a pull request as the facts the forge holds, and a call as the real call card. Every pane has one head with the way back, which Esc is too. Ported from
// proto/cockpit-flight-deck. #2301.

import { useEffect, useRef, type ReactNode } from "react";
import { ExternalLink, GitPullRequest, Rows3, Square, SquareTerminal, X, type LucideIcon } from "lucide-react";
import { Button, Select, Tooltip, isPressed, pressedDigit } from "@armada/components";
import { BayManage } from "@armada/cleanup";
import { holdsText } from "@armada/screens/src/keys";
import type { StartWhen } from "@armada/screens/src/draft/sessions";
import { START_WHEN, planPartsOf, startsFrom } from "@armada/screens/src/session-waits";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import { age, type Item } from "../../Dashboard";
import { useViewing } from "../viewed";
import { CallCard, type CardKeys } from "../CallCard";
import { dotsOf } from "../horizon";
import { ByHandActs } from "../../merge-by-hand";
import type { CockpitProps } from "./props";
import { viewsOf } from "../../merge-line";
import { SessionView } from "../../sessions";
import type { Hosts } from "../../session-view/SessionWorkspace";
import { OPEN_SESSION_ATTRIBUTE } from "../../open-session";
import { useSessions, useSessionsDraft } from "../../sessions-draft";
import type { Session, SessionsDraft, SessionWait } from "@armada/screens/src/draft/sessions";
import { restoreThread } from "../../session-view/place";
import type { Acts } from "./acts";
import { clockOf, heldFor, shortOf, spanOf, type BayView, type Holder as LedgerHolder, type Ledger } from "./bays";
import { useBayHistory } from "./bay-history";
import { SlotHistory } from "./SlotHistory";
import { MarkGlyph } from "./shared";
import type { Pr, Subject } from "./ship";
import { toneOf } from "./order";
import { answered, goBack, leaveAutopilot, openFrom, select, useDeck } from "./store";


/** The way back, which every pane's head ends on: to the bay it was opened from, else to the map or timeline. */
function BackAct() {
  const autopilot = useDeck((deck) => deck.sel?.kind === "autopilot");
  const from = useDeck((deck) => deck.from?.kind);
  const back = from === "session" ? "Back to the Job" : from !== undefined ? "Back to the bay" : "Back";
  return (
    <Tooltip label={back} shortcut="Esc">
      <Button variant="ghost" size="sm" iconOnly aria-label={back} onClick={() => (autopilot ? leaveAutopilot() : goBack())}>
        <X size={16} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}

/** The one head every pane has: what it is, its name, where it is, its acts, and the way back. */
export function PaneHead({ glyph: Glyph, kind, title, where, mark, children }: { glyph: LucideIcon; kind: string; title: string; where?: string | undefined; mark?: ReactNode; children?: ReactNode }) {
  return (
    <header className="deck-band deck-pane__head">
      <Tooltip label={kind}>
        <span className="deck-pane__kind" role="img" aria-label={kind}>
          <Glyph size={16} aria-hidden="true" />
        </span>
      </Tooltip>
      {mark}
      <h2 className="deck-pane__title">{title}</h2>
      {where === undefined ? null : <span className="deck-pane__where">{where}</span>}
      <span className="deck-pane__acts">
        {children}
        <BackAct />
      </span>
    </header>
  );
}

// ---- A Session -----------------------------------------------------------------------------------------

/**
 * A Session on the stage is the panel the Sessions window draws: its head with the state, the context figure and the rename, its
 * thread and its box. **Its ledger is drawn in the deck's right column**, in `ledger`, where the Bays were. The deck's own act
 * (Stop) leads the head's menu, Back is the head's far right item, and the pull request is the ledger's, not the head's.
 * `openSession` opens another Session where this one is, as the list does. No call strip: what the Session asks is in its own thread
 * and Needs you band.
 */
export function SessionPane({ subject, pull, props, acts, ledger, hosts, openSession }: { subject: Subject; pull?: number; props: CockpitProps; acts: Acts; ledger: HTMLElement | null; hosts: Hosts | undefined; openSession: (id: string) => void }) {
  const sessions = useSessions();
  const session = sessions.find((one) => one.id === subject.id);
  const draft = useSessionsDraft();
  // Open on the stage is being in it, as on the Sessions page: its reply is read, so its call does not come back.
  useViewing(session?.id);
  // Back from a Session it follows finds the thread where he left it.
  useEffect(() => {
    if (session?.id !== undefined) restoreThread(session.id);
  }, [session?.id]);
  const { state } = props;
  if (session === undefined || draft === undefined) return null;
  if (session.waits !== undefined) return <WaitingPane subject={subject} session={session} wait={session.waits} sessions={sessions} draft={draft} />;
  return (
    <section className="deck-pane" data-frame="" aria-label={`Job: ${subject.title}`} {...{ [OPEN_SESSION_ATTRIBUTE]: session.id }}>
      <SessionView
        session={session}
        goes={{ onGoTo: props.onGoTo ?? (() => {}), onOpenLink: props.onOpenLink, onOpenCheck: (id, run) => (props.onOpenCheck === undefined ? props.onOpenSession(id) : props.onOpenCheck(id, run)), onOpenLoadout: (id) => props.onOpenLoadout?.(id) }}
        onOpen={openSession}
        // The deck's ship already wants the pool while it is drawn.
        held={{ held: state.held, onWant: () => {} }}
        lines={state.mergeLines}
        settings={state.settings}
        repositories={state.holds.repositories ?? []}
        deck={{
          ledger,
          ...(pull === undefined ? {} : { pull }),
          ...(hosts === undefined ? {} : { hosts }),
          acts: [
            ...(subject.stoppable && acts.can.stop(subject) ? [{ id: "stop", label: "Stop", icon: Square, run: () => acts.stop(subject) }] : []),
          ],
          back: <BackAct />,
        }}
      />
    </section>
  );
}

// ---- A Session that waits on another ----------------------------------------------------------------------

/**
 * A Session dispatched after another and not started: its brief, the Session it waits on, and when it starts and from where. The
 * condition can be changed, and Start now ignores it. It has no thread to read and no box to answer in; those come when it starts.
 */
function WaitingPane({ subject, session, wait, sessions, draft }: { subject: Subject; session: Session; wait: SessionWait; sessions: readonly Session[]; draft: SessionsDraft }) {
  const parts = planPartsOf(sessions.find((one) => one.id === wait.on.id));
  const set = (when: StartWhen, part?: string) => {
    const found = parts.find((one) => one.id === part);
    draft.startWhen?.(session.id, when, found === undefined ? undefined : { id: found.id, title: found.title });
  };
  return (
    <section className="deck-pane" data-frame="" aria-label={`Job: ${subject.title}`} {...{ [OPEN_SESSION_ATTRIBUTE]: session.id }}>
      <PaneHead glyph={SquareTerminal} kind="Job" title={subject.title} where={subject.repo} mark={<MarkGlyph mark={{ ...subject.mark, tone: toneOf(subject) }} />}>
        {draft.startNow === undefined ? null : (
          <Button variant="primary" size="sm" onClick={() => draft.startNow?.(session.id)}>
            Start now
          </Button>
        )}
      </PaneHead>
      <div className="deck-pane__body deck-waiting">
        <dl className="deck-facts">
          <div className="deck-facts__row">
            <dt>Brief</dt>
            <dd className="deck-waiting__brief">{wait.brief}</dd>
          </div>
          <div className="deck-facts__row">
            <dt>Waits on</dt>
            <dd>
              <Holder id={wait.on.id} title={wait.on.title} />
            </dd>
          </div>
          <div className="deck-facts__row">
            <dt>Starts when</dt>
            <dd className="deck-waiting__when">
              {draft.startWhen === undefined ? (
                START_WHEN.find((one) => one.id === wait.when)?.label
              ) : (
                <>
                  <Select aria-label="Starts when" value={wait.when} onChange={(event) => set(event.target.value as StartWhen, event.target.value === "plan" ? parts[0]?.id : undefined)}>
                    {START_WHEN.map((one) => (
                      <option key={one.id} value={one.id} disabled={one.id === "plan" && parts.length === 0}>
                        {one.label}
                      </option>
                    ))}
                  </Select>
                  {wait.when !== "plan" ? null : (
                    <Select aria-label="Plan part" value={wait.part?.id ?? parts[0]?.id ?? ""} onChange={(event) => set("plan", event.target.value)}>
                      {parts.map((one) => (
                        <option key={one.id} value={one.id}>
                          {`${one.kind === "group" ? "Group" : "Task"}: ${one.title}`}
                        </option>
                      ))}
                    </Select>
                  )}
                </>
              )}
            </dd>
          </div>
          {wait.needs === "" ? null : (
            <div className="deck-facts__row">
              <dt>Needs</dt>
              <dd>{wait.needs}</dd>
            </div>
          )}
          <div className="deck-facts__row">
            <dt>Starts from</dt>
            <dd data-mono>{startsFrom(wait)}</dd>
          </div>
        </dl>
      </div>
    </section>
  );
}

// ---- Facts ---------------------------------------------------------------------------------------------

function Facts({ rows }: { rows: readonly (readonly [string, ReactNode, boolean?])[] }) {
  return (
    <dl className="deck-facts">
      {rows
        .filter(([, value]) => value !== undefined && value !== null && value !== "")
        .map(([term, value, mono]) => (
          <div key={term} className="deck-facts__row">
            <dt>{term}</dt>
            <dd data-mono={mono || undefined}>{value}</dd>
          </div>
        ))}
    </dl>
  );
}

/** A Session the facts name, as a link that opens it in this same area. */
function Holder({ id, title }: { id: string; title: string }) {
  return (
    <button type="button" className="deck-link" onClick={() => select({ kind: "session", key: `session:${id}` })}>
      <SquareTerminal size={12} aria-hidden="true" />
      <span>{title}</span>
    </button>
  );
}

// ---- A bay ---------------------------------------------------------------------------------------------

export function SlotPane({ view, ledger, clock, repo, known, props }: { view: BayView; ledger: Ledger; clock: number; repo: string; known: ReadonlySet<string>; props: CockpitProps }) {
  const slot = view.slot;
  const stranded = view.state === "stranded";
  const history = useBayHistory(slot.manifest_id, slot.slot, ledger);
  const sends = props.bayActs;
  const manages = sends !== undefined || props.onRescueSlot !== undefined;
  // A holder the ship still holds opens as a link; one that is gone is named by its id.
  const who = (one: LedgerHolder | undefined) =>
    one === undefined ? undefined : known.has(`session:${one.id}`) ? <Holder id={one.id} title={one.title ?? one.id} /> : <span className="deck-link__plain">{one.title ?? shortOf(one.id)}</span>;
  const mark = <MarkGlyph mark={{ Glyph: view.Glyph, said: view.said ?? view.word, tone: stranded ? "ask" : "calm" }} />;
  const files = slot.stranded?.uncommitted ?? [];
  const commits = slot.stranded?.commits ?? [];
  return (
    <section className="deck-pane" aria-label={`Bay: slot-${slot.slot}`}>
      <PaneHead glyph={Rows3} kind="Bay" title={`slot-${slot.slot}`} where={repo} mark={mark} />
      <div className="deck-pane__body">
        {!manages ? null : (
          <div className="deck-slot__manage" data-bay={slot.closed === true ? "closed" : view.state}>
            <BayManage
              key={view.key}
              slot={slot}
              now={props.now}
              onChange={sends?.onChange}
              onRescue={props.onRescueSlot}
            />
          </div>
        )}
        <Facts
          rows={[
            ["State", view.said ?? view.word],
            ["Holder", who(view.holder)],
            ["Branch", view.branch, true],
            ["Held for", view.heldFor === undefined ? undefined : spanOf(view.heldFor)],
            ["Given back", view.freed === undefined ? undefined : `${spanOf(view.freed)} ago`],
            ["Last holder", stranded ? (who(view.last) ?? "Holder gone") : undefined],
            ["Told", view.told === undefined ? undefined : clockOf(view.told)],
            ["Uncommitted", files.length === 0 ? undefined : files.join("\n"), true],
            ["Unpushed", commits.length === 0 ? undefined : commits.filter((one) => one.home === "only_here").map((one) => `${one.sha.slice(0, 7)} ${one.subject}`).join("\n"), true],
            ["Base", slot.base, true],
            ["Behind", (slot.behind ?? 0) > 0 ? `${slot.behind} commits` : undefined],
            ["Checkout", slot.warm ? "Warm" : "Cold"],
            ["Path", slot.path, true],
          ]}
        />
        {history === undefined ? null : <SlotHistory history={history} clock={clock} known={known} onOpen={(one) => openFrom({ kind: "session", key: `session:${one.id}` })} />}
      </div>
    </section>
  );
}

// ---- A pull request ------------------------------------------------------------------------------------

export function PrPane({ pr, owner, props, acts }: { pr: Pr; owner: Subject | undefined; props: CockpitProps; acts: Acts }) {
  const sessions = useSessions();
  const views = viewsOf(props.state);
  const view = views.find((one) => one.root === pr.root);
  const dot = view === undefined ? undefined : dotsOf(view, props.state.mergeLines, sessions).find((one) => one.key === `pull:${pr.number}`);
  const card = dot?.card;
  const opened = heldFor(card?.opened, props.now);
  const who = owner === undefined ? undefined : <Holder id={owner.id} title={owner.title} />;
  return (
    <section className="deck-pane" aria-label={`Pull request #${pr.number}`}>
      <PaneHead glyph={GitPullRequest} kind="Pull request" title={`#${pr.number} ${pr.title}`} where={view === undefined ? undefined : props.state.holds.repositories?.find((one) => one.root === pr.root)?.manifest?.repository}>
        {owner !== undefined && owner.mergeable && acts.can.merge(owner) ? (
          <Button variant="secondary" size="sm" aria-label={`Merge #${pr.number}`} onClick={() => acts.merge(owner)}>
            Merge
          </Button>
        ) : null}
        <Tooltip label="Open on the forge">
          <Button variant="ghost" size="sm" iconOnly aria-label="Open on the forge" onClick={() => props.onOpenLink(pr.url)}>
            <ExternalLink size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
      </PaneHead>
      <div className="deck-pane__body" data-tone={pr.stage === "red" ? "bad" : undefined}>
        <Facts
          rows={[
            ["State", card?.says],
            ["Checks", card?.check],
            ["Place", card?.place === undefined ? undefined : String(card.place)],
            ["Branch", pr.branch, true],
            ["Author", pr.author],
            ["Opened", opened === undefined ? undefined : `${opened} ago`],
            ["Review", card?.review],
            ["Merge", card?.byHand === true ? <ByHandActs root={pr.root} number={pr.number} ci={pr.ci} /> : undefined],
            ["Draft", card?.draft === true ? "Yes" : undefined],
            ["Conflicts", card?.conflicts === true ? "Yes" : undefined],
            ["Owner", who],
          ]}
        />
      </div>
    </section>
  );
}

// ---- A call --------------------------------------------------------------------------------------------

/**
 * The real call card, for a call that is about neither a Session nor a pull request, answered through the acts the
 * Cockpit's is. **The keys it answers to are the `call` scope's, bound while it is up**, and the deck's own stand down for it.
 */
export function CallPane({ item, props }: { item: Item; props: CockpitProps }) {
  const answering = useRef<CardKeys | undefined>(undefined);
  const finish = () => {
    answered(item.key);
    select(undefined);
  };
  const later = () => select(undefined);
  useListKeydown((event) => {
    if (event.repeat) return;
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    if (holdsText(event.target)) {
      const field = event.target;
      if (field instanceof HTMLInputElement && field.value === "" && (isPressed("close", event) || (bare && event.key === "ArrowDown"))) {
        event.preventDefault();
        field.blur();
      }
      return;
    }
    const card = answering.current;
    if (card === undefined) return;
    const claim = () => event.preventDefault();
    const digit = pressedDigit("call_pick", event);
    if (digit !== null) return claim(), card.pickNumber(digit);
    if (card.pickKey(event)) return claim();
    if (isPressed("call_reply", event) && card.change !== undefined) return claim(), card.change();
    if (isPressed("call_reply", event)) {
      const field = document.querySelector<HTMLTextAreaElement>(".deck-call .armada-callcard__reply textarea");
      return field === null ? undefined : (claim(), field.focus());
    }
    if (isPressed("call_send", event)) {
      if ((event.target as HTMLElement).tagName === "BUTTON" && document.querySelector(".deck-call .armada-callcard")?.contains(event.target as Node) === true) return;
      return card.canSend ? (claim(), card.send()) : undefined;
    }
    if (isPressed("call_later", event) || isPressed("close", event)) return claim(), card.pending === undefined ? later() : undefined;
    if (isPressed("call_dismiss", event)) return claim(), card.dismiss();
    if (isPressed("call_open_walk", event)) return card.openWalk === undefined ? undefined : (claim(), card.openWalk());
    if (isPressed("open", event)) return card.open === undefined ? undefined : (claim(), card.open());
  });
  const Icon = item.icon;
  return (
    <section className="deck-pane" aria-label={`Call: ${item.title}`} data-hue={item.hue}>
      <PaneHead glyph={Icon} kind={item.kind} title={item.title} where={[item.where, age(item.at, props.now)].filter((one) => one !== "").join(" · ")} />
      <div className="deck-call armada-cockpit__scrim">
        <div className="armada-stack" data-consoled style={{ ["--behind" as string]: 0 }}>
          <CallCard key={item.key} item={item} now={props.now} state={props.state} hosts={props} finish={finish} later={later} dismiss={finish} leaving={undefined} from={undefined} answering={answering} />
        </div>
      </div>
    </section>
  );
}
