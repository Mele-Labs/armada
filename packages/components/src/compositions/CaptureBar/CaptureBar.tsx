import { Check } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";

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
  /** What the Manifest calls the server this window is on. */
  run: string;
  /** Scheme, host and port, drawn in full and never abbreviated. */
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
};

/** Which panel the window has, whether it is drawn, and the press that changes that. */
export type CaptureBarPanel = { kind: "session" | "job"; open: boolean; onToggle: (open: boolean) => void };

/** Whether the bar has a second row to draw — a refusal, the run having ended, or a folded question. */
export function hasASecondRow(
  props: Pick<CaptureBarProps, "serving" | "refused" | "framesRefused" | "question">,
): boolean {
  return !props.serving || props.refused !== undefined || props.framesRefused > 0 || props.question !== undefined;
}

export function CaptureBar(props: CaptureBarProps) {
  const { run, address, studio, job, session, serving, armed, framesRefused, refused, binding, approval, question, panel } = props;
  const owner = job ?? session;
  const landsOn = studio !== null ? `Notes land on ${studio}` : owner !== undefined ? `Notes go to ${owner}` : null;
  return (
    <div className="armada-capture-bar" data-armed={armed ? "" : undefined}>
      <div className="armada-capture-bar__row">
        <span className="armada-capture-bar__run" title="This window shows a server, not Armada">
          {run}
        </span>
        <span className="armada-capture-bar__address">{address}</span>
        <span className="armada-capture-bar__aim">
          {!serving ? "The run ended" : (landsOn ?? "Walking a Job's server")}
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
            disabled={approval?.state === "approved"}
            pending={approval?.state === "pending"}
            {...(approval?.state === "refused" ? { answer: "refused" as const, title: approval.said } : {})}
            onClick={props.onApprove}
          >
            <Check size={12} strokeWidth={2} aria-hidden />
            {approval?.state === "approved" ? "Approved" : "Approve"}
          </Button>
        )}
        {panel === undefined ? null : (
          <Button variant="ghost" size="sm" aria-expanded={panel.open} onClick={() => panel.onToggle(!panel.open)}>
            {`${panel.open ? "Hide" : "Show"} ${panel.kind === "session" ? "Session" : "Job"}`}
          </Button>
        )}
      </div>
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
