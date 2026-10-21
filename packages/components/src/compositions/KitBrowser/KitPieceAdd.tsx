import { useId, useState, type FormEvent, type ReactNode } from "react";
import type { KitPieceKind, KitSourceFound } from "@armada/protocol";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";

/** What a look at a source came to. */
export type KitLookAnswer = { state: "found"; found: KitSourceFound } | { state: "refused"; why: ReactNode };

/** What an install came to. A command that failed is drawn in the form, which stays open on it. */
export type KitInstallAnswer =
  | { state: "installed" }
  | { state: "failed"; command: string; exitCode: number; error: string }
  | { state: "refused"; why: ReactNode };

/** What starting a Job for a source came to, by the title it was started under. */
export type KitJobAnswer = { state: "started"; title: string } | { state: "refused"; why: ReactNode };

/** Add and Remove for skills and plugins. */
export type KitPiecesProps = {
  /** What a source is, read before anything runs. The commands Install runs come with the answer. */
  onLook: (source: string) => Promise<KitLookAnswer>;
  /** A Job that takes the source, sets it up and reports back. */
  onStartJob: (source: string) => Promise<KitJobAnswer>;
  /** `source` is the repository the look resolved to. */
  onInstall: (adding: { source: string; kind: KitPieceKind; manifest: boolean }) => Promise<KitInstallAnswer>;
  onRemove: (piece: { kind: KitPieceKind; name: string }) => void;
  /** The skill last moved to the Trash, while Undo can bring it back. */
  trashed: { name: string } | null;
  onUndo: () => void;
  /** Why the last Remove or Undo did nothing. */
  refused?: ReactNode;
};

type Look = { state: "idle" } | { state: "looking" } | KitLookAnswer;
type Failed = Exclude<KitInstallAnswer, { state: "installed" }>;

/**
 * The form Add opens in the detail. One field takes the source; Look asks Fleet what it is and draws the answer, with
 * the commands Install would run, before anything runs. Starting a Job is offered from the press of Look on, so a
 * source Fleet cannot place, or a look that does not answer, still has somewhere to go.
 */
export function KitPieceAdd({ keep, pieces, onInstalled }: { keep: string; pieces: KitPiecesProps; onInstalled: () => void }) {
  const group = useId();
  const [source, setSource, clearSource] = useKept(`${keep}:source`, "");
  const [look, setLook] = useState<Look>({ state: "idle" });
  const [manifest, setManifest] = useState(true);
  const [failed, setFailed] = useState<Failed | null>(null);
  const [installing, setInstalling] = useState(false);
  const [job, setJob] = useState<KitJobAnswer | null>(null);
  const typed = source.trim();
  const found = look.state === "found" ? look.found : undefined;
  const placed = found === undefined || found.found === "unknown" ? undefined : found;

  async function lookAt(event: FormEvent): Promise<void> {
    event.preventDefault();
    if (typed === "" || look.state === "looking") return;
    setLook({ state: "looking" });
    setFailed(null);
    setJob(null);
    setLook(await pieces.onLook(typed));
  }

  async function install(): Promise<void> {
    if (placed === undefined || installing) return;
    setInstalling(true);
    setFailed(null);
    const answer = await pieces.onInstall({ source: placed.repository, kind: placed.found, manifest });
    setInstalling(false);
    if (answer.state !== "installed") {
      setFailed(answer);
      return;
    }
    clearSource();
    onInstalled();
  }

  async function startJob(): Promise<void> {
    setJob(await pieces.onStartJob(typed));
  }

  const jobAct = (primary: boolean) => (
    <Button type="button" variant={primary ? "primary" : "secondary"} size="sm" ground="card" disabled={job?.state === "started"} onClick={() => void startJob()}>
      Start a Job to set it up
    </Button>
  );

  return (
    <div className="armada-kit-browser__form">
      <h3 className="armada-kit-browser__title">Add</h3>
      <form className="armada-kit-browser__look" onSubmit={(event) => void lookAt(event)}>
        <Input
          id={`${group}-source`}
          label="Source"
          value={source}
          placeholder="owner/repository, or a link"
          spellCheck={false}
          onChange={(event) => {
            setSource(event.target.value);
            setLook({ state: "idle" });
            setFailed(null);
            setJob(null);
          }}
        />
        <div className="armada-kit-browser__acts">
          <Button type="submit" variant={look.state === "idle" || look.state === "refused" ? "primary" : "secondary"} size="sm" ground="card" disabled={typed === "" || look.state === "looking"}>
            {look.state === "looking" ? "Looking" : "Look"}
          </Button>
        </div>
      </form>

      {look.state === "refused" ? <p className="armada-kit-browser__refused">{look.why}</p> : null}
      {found === undefined ? null : <Found found={found} />}

      {placed === undefined ? null : (
        <>
          <section className="armada-kit-browser__runs" aria-label="Runs">
            <h4 className="armada-kit-browser__caption">Runs</h4>
            <pre className="armada-kit-browser__commands">{placed.commands.join("\n")}</pre>
          </section>
          <Checkbox checked={manifest} onChange={(event) => setManifest(event.target.checked)}>
            Also add to my Kit manifest
          </Checkbox>
        </>
      )}

      {failed === null ? null : failed.state === "refused" ? (
        <p className="armada-kit-browser__refused">{failed.why}</p>
      ) : (
        <section className="armada-kit-browser__failed" aria-label="Install failed">
          <p className="armada-kit-browser__refused">
            <code>{failed.command}</code> exited {failed.exitCode}
          </p>
          <pre className="armada-kit-browser__error">{failed.error}</pre>
        </section>
      )}

      {look.state === "idle" ? null : (
        <div className="armada-kit-browser__acts">
          {placed === undefined ? null : (
            <Button type="button" variant="primary" size="sm" ground="card" disabled={installing} onClick={() => void install()}>
              {installing ? "Installing" : "Install"}
            </Button>
          )}
          {jobAct(found?.found === "unknown")}
        </div>
      )}
      {job === null ? null : job.state === "refused" ? (
        <p className="armada-kit-browser__refused">{job.why}</p>
      ) : (
        <p className="armada-kit-browser__started" role="status">
          Started a Job: {job.title}
        </p>
      )}
    </div>
  );
}

/** What the look found, as facts. A plugin lists the skills it brings by name. */
function Found({ found }: { found: KitSourceFound }) {
  if (found.found === "unknown") {
    return (
      <section className="armada-kit-browser__found" aria-label="Found">
        <p className="armada-kit-browser__found-head">No plugin or skill found</p>
        <p className="armada-kit-browser__comes">{found.why}</p>
      </section>
    );
  }
  return (
    <section className="armada-kit-browser__found" aria-label="Found">
      <p className="armada-kit-browser__found-head">
        <span>{found.name}</span>
        <span className="armada-kit-browser__mark">{found.found === "plugin" ? "Plugin" : "Skill"}</span>
      </p>
      <dl className="armada-kit-browser__facts">
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
