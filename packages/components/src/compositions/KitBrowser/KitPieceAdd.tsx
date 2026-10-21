import { useId, useState, type FormEvent, type ReactNode } from "react";
import type { KitPieceKind } from "@armada/protocol";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";

/** What an install came to. A command that failed is drawn in the form, which stays open on it. */
export type KitInstallAnswer =
  | { state: "installed" }
  | { state: "failed"; command: string; exitCode: number; error: string }
  | { state: "refused"; why: ReactNode };

/** Add and Remove for skills and plugins. */
export type KitPiecesProps = {
  /** The commands Add will run for a source, in order, shown before anything runs. */
  commandsFor: (source: string, kind: KitPieceKind) => string[];
  /** The command Remove runs on a plugin, by its name as the inventory spells it. */
  removeCommand: (name: string) => string;
  onInstall: (adding: { source: string; kind: KitPieceKind; manifest: boolean }) => Promise<KitInstallAnswer>;
  onRemove: (piece: { kind: KitPieceKind; name: string }) => void;
  /** The skill last moved to the Trash, while Undo can bring it back. */
  trashed: { name: string } | null;
  onUndo: () => void;
  /** Why the last Remove or Undo did nothing. */
  refused?: ReactNode;
};

type Failed = Exclude<KitInstallAnswer, { state: "installed" }>;

/** The form Add opens in the detail: a source, Plugin or Skill, the commands that will run, and the manifest checkbox. */
export function KitPieceAdd({ keep, first, pieces, onInstalled }: { keep: string; first: KitPieceKind; pieces: KitPiecesProps; onInstalled: () => void }) {
  const group = useId();
  const [source, setSource, clearSource] = useKept(`${keep}:source`, "");
  const [kind, setKind] = useState<KitPieceKind>(first);
  const [manifest, setManifest] = useState(true);
  const [failed, setFailed] = useState<Failed | null>(null);
  const [running, setRunning] = useState(false);
  const typed = source.trim();
  const commands = typed === "" ? [] : pieces.commandsFor(typed, kind);

  async function add(event: FormEvent): Promise<void> {
    event.preventDefault();
    if (typed === "" || running) return;
    setRunning(true);
    setFailed(null);
    const answer = await pieces.onInstall({ source: typed, kind, manifest });
    setRunning(false);
    if (answer.state !== "installed") {
      setFailed(answer);
      return;
    }
    clearSource();
    onInstalled();
  }

  return (
    <form className="armada-kit-browser__form" onSubmit={(event) => void add(event)}>
      <h3 className="armada-kit-browser__title">{kind === "plugin" ? "Add plugin" : "Add skill"}</h3>
      <Input
        id={`${group}-source`}
        label="Source"
        value={source}
        placeholder="owner/repository"
        spellCheck={false}
        onChange={(event) => {
          setSource(event.target.value);
          setFailed(null);
        }}
      />
      <RadioGroup label="Install as">
        {(["plugin", "skill"] as const).map((one) => (
          <Radio
            key={one}
            name={`${group}-kind`}
            value={one}
            checked={kind === one}
            onChange={() => {
              setKind(one);
              setFailed(null);
            }}
          >
            {one === "plugin" ? "Plugin" : "Skill"}
          </Radio>
        ))}
      </RadioGroup>
      {commands.length === 0 ? null : (
        <section className="armada-kit-browser__runs" aria-label="Runs">
          <h4 className="armada-kit-browser__caption">Runs</h4>
          <pre className="armada-kit-browser__commands">{commands.join("\n")}</pre>
        </section>
      )}
      <Checkbox checked={manifest} onChange={(event) => setManifest(event.target.checked)}>
        Also add to my Kit manifest
      </Checkbox>
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
      <div className="armada-kit-browser__acts">
        <Button type="submit" variant="primary" size="sm" ground="card" disabled={typed === "" || running}>
          {running ? "Adding" : "Add"}
        </Button>
      </div>
    </form>
  );
}
