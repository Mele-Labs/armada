import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import type { KitPieceKind, KitSourceFound } from "@armada/protocol";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Sheet } from "../../primitives/Sheet/Sheet";

/** What a look at a source came to. */
export type KitLookAnswer = { state: "found"; found: KitSourceFound } | { state: "refused"; why: ReactNode };

/** What an install came to. A command that failed is drawn in the panel, which stays open on it. */
export type KitInstallAnswer =
  | { state: "installed" }
  | { state: "failed"; command: string; exitCode: number; error: string; takenBack: string[] }
  | { state: "refused"; why: ReactNode };

/** What starting a Job, or sending to one, came to: the Job's title, or why not. */
/** What starting a Job, or sending to one, came to: the Job, by id and title where it has one yet, or why not. */
export type LinkJobAnswer = { state: "done"; id: string; title?: string | undefined } | { state: "refused"; why: ReactNode };

export type LinkPanelProps = {
  open: boolean;
  /** The link it opened on, looked at straight away. Empty for an Add pressed with nothing dropped. */
  source: string;
  onClose: () => void;
  /** What a source is, read before anything runs. The commands Install runs come with the answer. */
  onLook: (source: string) => Promise<KitLookAnswer>;
  /** `source` is the repository the look resolved to. */
  onInstall: (adding: { source: string; kind: KitPieceKind; manifest: boolean }) => Promise<KitInstallAnswer>;
  /** A Job that takes the source, sets it up and reports back. With `failed`, it is asked to fix that install. */
  onStartJob: (source: string, failed?: { command: string; error: string }) => Promise<LinkJobAnswer>;
  /** The Job open in the window, which Add to this Job sends the link to as a message from the person. Absent offers no such choice. */
  job?: { title?: string | undefined; onAdd: (source: string, found: KitSourceFound | undefined) => Promise<LinkJobAnswer> } | undefined;
  /** Opens a Job this panel started or sent to, closing the panel. */
  onOpenJob: (id: string) => void;
  /** A Job's title as the window holds it now, for a Job that had none when it started. */
  titleOf: (id: string) => string | undefined;
};

type Look = { state: "idle" } | { state: "looking" } | KitLookAnswer;

/** How long typing in Source rests before what it holds is looked at. */
const SETTLE_MS = 600;

/** Whether the field holds something a look could place: a link, or `owner/repository`. */
const couldBeSource = (text: string): boolean => /^https?:\/\/\S+\.\S+$/i.test(text) || /^[\w.-]+\/[\w.-]+$/.test(text);
type Failed = Exclude<KitInstallAnswer, { state: "installed" }>;

/**
 * A link dropped anywhere in Bridge, and the three things it can become: installed in Kit, a Job of its own, or a
 * message to the Job open. **One panel**: Kit's own Add opens it too, empty. It looks at a link as it opens, and at
 * what is typed once typing rests, and draws what the source is before anything runs. Starting a Job is offered from the press of Look on, so a source Fleet
 * cannot place, or a look that does not answer, still has somewhere to go.
 */
export function LinkPanel(props: LinkPanelProps) {
  return (
    <Sheet open={props.open} title="Add a link" kind="link-panel" onClose={props.onClose}>
      {props.open ? <LinkChoices {...props} /> : null}
    </Sheet>
  );
}

function LinkChoices({ source: dropped, onLook, onInstall, onStartJob, job, onOpenJob, titleOf }: LinkPanelProps) {
  const said = (answer: LinkJobAnswer | null, done: string) => <JobSaid answer={answer} done={done} onOpen={onOpenJob} titleOf={titleOf} />;
  const group = useId();
  const [source, setSource] = useKept("link-panel:source", dropped);
  const [look, setLook] = useState<Look>({ state: "idle" });
  const [manifest, setManifest] = useState(true);
  const [failed, setFailed] = useState<Failed | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);
  const [started, setStarted] = useState<LinkJobAnswer | null>(null);
  const [sent, setSent] = useState<LinkJobAnswer | null>(null);
  const [fixing, setFixing] = useState<LinkJobAnswer | null>(null);
  const typed = source.trim();
  const found = look.state === "found" ? look.found : undefined;
  const placed = found === undefined || found.found === "unknown" ? undefined : found;

  const reset = () => {
    setFailed(null);
    setFixing(null);
    setStarted(null);
    setSent(null);
    setInstalled(false);
  };

  // Each look takes a number, and only the newest one's answer is drawn: a late answer for older text never is.
  const latest = useRef(0);
  const looked = useRef("");
  async function lookAt(at: string): Promise<void> {
    const mine = ++latest.current;
    looked.current = at;
    setLook({ state: "looking" });
    reset();
    const answer = await onLook(at);
    if (mine === latest.current) setLook(answer);
  }

  // Each open starts from what opened it: a dropped link is looked at at once, and Add starts empty.
  useEffect(() => {
    setSource(dropped);
    if (dropped !== "") void lookAt(dropped);
  }, [dropped]);

  // What is typed is looked at once typing rests, where it could be a source and was not the last thing looked at.
  useEffect(() => {
    if (!couldBeSource(typed) || typed === looked.current) return;
    const timer = window.setTimeout(() => void lookAt(typed), SETTLE_MS);
    return () => window.clearTimeout(timer);
  }, [typed]);

  async function install(): Promise<void> {
    if (placed === undefined || installing) return;
    setInstalling(true);
    setFailed(null);
    setFixing(null);
    const answer = await onInstall({ source: placed.repository, kind: placed.found, manifest });
    setInstalling(false);
    if (answer.state === "installed") setInstalled(true);
    else setFailed(answer);
  }

  const unknown = found?.found === "unknown";

  return (
    <div className="armada-link-panel">
      <div className="armada-link-panel__look">
        <Input
          id={`${group}-source`}
          label="Source"
          value={source}
          placeholder="A link, or owner/repository"
          spellCheck={false}
          onChange={(event) => {
            // Any answer still to come is for older text.
            latest.current += 1;
            looked.current = "";
            setSource(event.target.value);
            setLook({ state: "idle" });
            reset();
          }}
        />
        {look.state === "looking" ? (
          <p className="armada-link-panel__why" role="status">
            Looking
          </p>
        ) : null}
      </div>

      {look.state === "refused" ? <p className="armada-link-panel__refused">{look.why}</p> : null}
      {found === undefined ? null : <Found found={found} />}

      {placed === undefined ? null : (
        <section className="armada-link-panel__choice" aria-label="Install in Kit">
          <h4 className="armada-link-panel__caption">Install in Kit</h4>
          <section aria-label="Runs">
            <pre className="armada-link-panel__commands">{placed.commands.join("\n")}</pre>
          </section>
          <Checkbox checked={manifest} onChange={(event) => setManifest(event.target.checked)}>
            Also add to my Kit manifest
          </Checkbox>
          {failed === null ? null : failed.state === "refused" ? (
            <p className="armada-link-panel__refused">{failed.why}</p>
          ) : (
            <section className="armada-link-panel__failed" aria-label="Install failed">
              <p className="armada-link-panel__refused">
                <code>{failed.command}</code> exited {failed.exitCode}
              </p>
              <pre className="armada-link-panel__commands">{failed.error}</pre>
              {failed.takenBack.length === 0 ? null : <p className="armada-link-panel__why">Fleet took back {failed.takenBack.join(", ")}.</p>}
              <div className="armada-link-panel__acts">
                <Button
                  type="button"
                  variant="primary"
                  size="sm"
                  ground="card"
                  disabled={fixing?.state === "done"}
                  onClick={() => void onStartJob(typed, { command: failed.command, error: failed.error }).then(setFixing)}
                >
                  Start a Job to fix it
                </Button>
                <Button type="button" variant="secondary" size="sm" ground="card" disabled={installing} onClick={() => void install()}>
                  {installing ? "Installing" : "Try again"}
                </Button>
                {said(fixing, "Started a Job")}
              </div>
            </section>
          )}
          <div className="armada-link-panel__acts">
            {failed?.state === "failed" ? null : (
              <Button type="button" variant="primary" size="sm" ground="card" disabled={installing || installed} onClick={() => void install()}>
                {installing ? "Installing" : "Install in Kit"}
              </Button>
            )}
            {installed ? (
              <span className="armada-link-panel__done" role="status">
                Installed {placed.name}
              </span>
            ) : null}
          </div>
        </section>
      )}

      {look.state === "idle" ? null : (
        <section className="armada-link-panel__choice" aria-label="Jobs">
          <div className="armada-link-panel__acts">
            <Button
              type="button"
              variant={unknown ? "primary" : "secondary"}
              size="sm"
              ground="card"
              disabled={started?.state === "done"}
              onClick={() => void onStartJob(typed).then(setStarted)}
            >
              Start a Job with it
            </Button>
            {said(started, "Started a Job")}
          </div>
          {job === undefined ? null : (
            <div className="armada-link-panel__acts">
              <Button type="button" variant="secondary" size="sm" ground="card" disabled={sent?.state === "done"} onClick={() => void job.onAdd(typed, found).then(setSent)}>
                Add to this Job
              </Button>
              {sent === null ? <span className="armada-link-panel__job">{job.title}</span> : said(sent, "Sent to the Job")}
            </div>
          )}
        </section>
      )}
    </div>
  );
}

/** What a start or a send came to, and the way to the Job it reached. */
function JobSaid({ answer, done, onOpen, titleOf }: { answer: LinkJobAnswer | null; done: string; onOpen: (id: string) => void; titleOf: (id: string) => string | undefined }) {
  if (answer === null) return null;
  if (answer.state === "refused") return <span className="armada-link-panel__refused">{answer.why}</span>;
  const title = titleOf(answer.id) ?? answer.title;
  return (
    <>
      <span className="armada-link-panel__done" role="status">
        {title === undefined ? done : `${done}: ${title}`}
      </span>
      <Button type="button" variant="ghost" size="sm" ground="card" onClick={() => onOpen(answer.id)}>
        Open the Job
      </Button>
    </>
  );
}

/** What the look found, as facts. A plugin lists the skills it brings by name. */
function Found({ found }: { found: KitSourceFound }) {
  if (found.found === "unknown") {
    return (
      <section className="armada-link-panel__found" aria-label="Found">
        <p className="armada-link-panel__found-head">No plugin or skill found</p>
        <p className="armada-link-panel__why">{found.why}</p>
      </section>
    );
  }
  return (
    <section className="armada-link-panel__found" aria-label="Found">
      <p className="armada-link-panel__found-head">
        <span>{found.name}</span>
        <span className="armada-link-panel__mark">{found.found === "plugin" ? "Plugin" : "Skill"}</span>
      </p>
      <dl className="armada-link-panel__facts">
        <div>
          <dt>Repository</dt>
          <dd data-mono="">{found.repository}</dd>
        </div>
        {found.found !== "plugin" ? null : (
          <>
            <div>
              <dt>Marketplace</dt>
              <dd data-mono="">{found.marketplace}</dd>
            </div>
            <div>
              <dt>Skills</dt>
              <dd>{found.skills.length === 0 ? "None" : found.skills.join(", ")}</dd>
            </div>
          </>
        )}
      </dl>
    </section>
  );
}
