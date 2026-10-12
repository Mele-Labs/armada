import { Check } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";
import { SessionMark } from "../SessionFrame/SessionFrame";

/**
 * The bar Bridge owns above another repository's running web app —
 * `docs/practices/capture-window.md`, *What a person sees*; `#1294`.
 *
 * **A person has to tell at a glance that what is below this is not Armada.**
 * So it names the Run, the address in full and the Studio a Note will land on,
 * and it is drawn outside the page rather than in it — a bar composited above
 * is a bar the page cannot draw over.
 *
 * **There is no address field, no Back and no Forward.** This is not a browser:
 * the window is pinned to one origin and Reload reloads that one.
 */

/**
 * Where an Approve press stands. `refused` carries Fleet's words, drawn as the
 * button's tooltip so the bar keeps its one row.
 */
export type CaptureBarApproval = { state: "pending" } | { state: "approved" } | { state: "refused"; said: string };

/** An address this window refused, as the bar says it. */
export type CaptureBarRefusal = {
  address: string;
  /** What moved: a navigation, a redirect, a popup or a download. */
  said: string;
  /** Whether the system browser would take it. */
  offerable: boolean;
};

export type CaptureBarProps = {
  /**
   * What the Manifest calls the server this window is on. **Absent where the window has no name of
   * its own** and was titled by its address: the address says it, and the bar does not say it twice.
   */
  run?: string | undefined;
  /** Scheme, host and port, drawn in full and never abbreviated. The whole address where there is no `run`. */
  address: string;
  /**
   * The Studio a Note lands on, by name. **`null`: a Job's server, opened to be
   * walked** — the bar says so, and offers no Capture, since a Note would have
   * nowhere to land.
   */
  studio: string | null;
  /**
   * The Job a note goes to where there is no Studio — the one whose server
   * this window walks. Capture is offered for it, and the note waits on the
   * Job until it is sent back.
   */
  job?: string;
  /** The Session a note goes to, where a Session showed this page. Its own prop, never `job`'s. */
  session?: string;
  /**
   * Open the Job or the Session named in "Notes go to", in Bridge's main window. **Absent: the name
   * is plain text.**
   */
  onOpenOwner?: () => void;
  /** Whether the Run is still serving. Capture ends with it. */
  serving: boolean;
  /** Whether capture is armed. */
  armed: boolean;
  /** How many subframe navigations were refused. Counted, never named. */
  framesRefused: number;
  refused?: CaptureBarRefusal;
  onArm: (on: boolean) => void;
  onReload: () => void;
  /** Hand the refused address to the browser this person already uses. */
  onFollowRefused: () => void;
  /** The binding that arms capture, from the action registry. */
  binding: readonly string[];
  /**
   * Tell the Job or the Session that opened this window that the person walked
   * it and it is right. **Absent: no Approve**, as on a Studio's window, which
   * has no owner to tell.
   */
  onApprove?: () => void;
  approval?: CaptureBarApproval;
  /**
   * A question the Session asked about this page, folded out of the way so the page can be used. The
   * second row says it is waiting and `onShowQuestion` draws it over the page again.
   */
  question?: string;
  onShowQuestion?: () => void;
  /**
   * The panel beside the page, holding the Session's thread or the Job that opened this window. **Absent:
   * a Studio's window, which has none.** One press draws it or folds it away.
   */
  panel?: CaptureBarPanel;
  /**
   * The takes a Session showed of one change, each a page of its own. One is drawn at a time; a
   * press swaps the page, and Approve and every note name the one on screen. **Absent: one page.**
   */
  variations?: CaptureBarVariations;
  /**
   * Whether the page is ready to walk, as the Session or the Job that opened it says: `ready` it is
   * done and waiting on the person, `working` it is still changing it. Drawn as the Session mark at
   * the bar's start and as the bar's own hue. **Absent: nobody has said.**
   */
  readiness?: CaptureBarReadiness;
};

export type CaptureBarReadiness = "ready" | "working";

/** Each readiness as the Session mark draws it, and what its tooltip says. */
const READINESS: Record<CaptureBarReadiness, { mark: "waiting" | "working"; said: string }> = {
  ready: { mark: "waiting", said: "Ready to walk" },
  working: { mark: "working", said: "Still changing" },
};

/** Each take's name in the order shown, which is on screen, and the press that swaps it. */
export type CaptureBarVariations = { names: readonly string[]; at: number; onPick: (at: number) => void };

/** A take's letter: A for the first. */
export const variationLetter = (at: number): string => String.fromCharCode(65 + at);

/** Which panel the window has, whether it is drawn, and the press that changes that. */
export type CaptureBarPanel = { kind: "session" | "job"; open: boolean; onToggle: (open: boolean) => void };

/** Whether the bar has a second row to draw — a refusal, the run having ended, or a folded question. */
export function hasASecondRow(
  props: Pick<CaptureBarProps, "serving" | "refused" | "framesRefused" | "question">,
): boolean {
  return !props.serving || props.refused !== undefined || props.framesRefused > 0 || props.question !== undefined;
}

export function CaptureBar(props: CaptureBarProps) {
  const { run, address, studio, job, session, serving, armed, framesRefused, refused, binding, approval, question, panel, variations, readiness } = props;
  const taking = variations === undefined ? "" : ` ${variationLetter(variations.at)}`;
  const owner = job ?? session;
  const landsOn = studio !== null ? `Notes land on ${studio}` : owner !== undefined ? `Notes go to ${owner}` : null;
  const aim =
    studio === null && owner !== undefined && props.onOpenOwner !== undefined ? (
      <>
        Notes go to{" "}
        <button
          type="button"
          className="armada-capture-bar__owner"
          title={job === undefined ? "Open this Session in Bridge" : "Open this Job in Bridge"}
          onClick={props.onOpenOwner}
        >
          {owner}
        </button>
      </>
    ) : (
      landsOn
    );
  return (
    <div className="armada-capture-bar" data-armed={armed ? "" : undefined} data-readiness={readiness}>
      <div className="armada-capture-bar__row">
        {readiness === undefined ? null : <SessionMark state={READINESS[readiness].mark} said={READINESS[readiness].said} />}
        {run === undefined ? null : (
          <span className="armada-capture-bar__run" title="This window shows a server, not Armada">
            {run}
          </span>
        )}
        <span className="armada-capture-bar__address">{address}</span>
        <span className="armada-capture-bar__aim">
          {!serving ? "The run ended" : (aim ?? "Walking a Job's server")}
        </span>
        <Button variant="ghost" size="sm" disabled={!serving} onClick={props.onReload}>
          Reload
        </Button>
        {landsOn === null ? null : (
          <>
            <Button
              variant={armed ? "primary" : "ghost"}
              size="sm"
              disabled={!serving}
              onClick={() => props.onArm(!armed)}
            >
              {armed ? "Capturing" : "Capture"}
            </Button>
            <KbdChord keys={[...binding]} aria-label={`${binding.join(" ")} turns capturing on`} />
          </>
        )}
        {props.onApprove === undefined ? null : (
          <Button
            variant="secondary"
            ground="sunken"
            size="sm"
            // While the opener is still changing the page there is nothing settled to approve.
            disabled={approval?.state === "approved" || readiness === "working"}
            pending={approval?.state === "pending"}
            {...(approval?.state === "refused" ? { answer: "refused" as const, title: approval.said } : {})}
            onClick={props.onApprove}
          >
            {readiness === "working" ? null : <Check size={12} strokeWidth={2} aria-hidden />}
            {readiness === "working"
              ? "Not ready"
              : approval?.state === "approved"
                ? `Approved${taking}`
                : `Approve${taking}`}
          </Button>
        )}
        {panel === undefined ? null : (
          <Button variant="ghost" size="sm" aria-expanded={panel.open} onClick={() => panel.onToggle(!panel.open)}>
            {`${panel.open ? "Hide" : "Show"} ${panel.kind === "session" ? "Session" : "Job"}`}
          </Button>
        )}
      </div>
      {/* A row of their own, so the takes never push the acts off a narrow window's bar. */}
      {variations === undefined ? null : (
        <div role="radiogroup" aria-label="Variations" className="armada-capture-bar__row armada-capture-bar__takes">
          {variations.names.map((name, at) => (
            <button
              key={at}
              type="button"
              role="radio"
              aria-checked={at === variations.at}
              className="armada-capture-bar__take"
              onClick={() => variations.onPick(at)}
            >
              <span className="armada-capture-bar__take-letter">{variationLetter(at)}</span>
              {name}
            </button>
          ))}
        </div>
      )}
      {hasASecondRow(props) ? (
        <div className="armada-capture-bar__said" role="status">
          {serving ? null : (
            <span className="armada-capture-bar__ended">
              This run is no longer serving. What is on screen stays and nothing further loads
              {landsOn === null ? "." : ", and capture is closed."}
            </span>
          )}
          {refused === undefined ? null : (
            <>
              <span className="armada-capture-bar__refused">
                {refused.said} refused: {refused.address}
              </span>
              {refused.offerable ? (
                <Button variant="ghost" size="sm" onClick={props.onFollowRefused}>
                  Open in browser
                </Button>
              ) : null}
            </>
          )}
          {question === undefined ? null : (
            <>
              <span className="armada-capture-bar__question">Question waiting: {question}</span>
              <Button variant="secondary" ground="sunken" size="sm" onClick={props.onShowQuestion}>
                Show question
              </Button>
            </>
          )}
          {framesRefused > 0 ? (
            <span className="armada-capture-bar__frames">
              {framesRefused} {framesRefused === 1 ? "frame" : "frames"} off this origin refused
            </span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
