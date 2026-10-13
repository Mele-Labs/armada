import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { COPY_DEBUG_INFO } from "../../errors/ErrorNotice/payload";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { ComposerNumbered, ComposerTag, SentFromComposer } from "../SessionComposer/SessionComposer";

/** One repository the switch may point Helm at — id is the Manifest id. */
export type HelmRepositoryOption = { id: string; label: string };

/** A recent row's id prefix: a repository listed twice must not repeat an id, and the prefix says which row. */
const RECENT = "recent:";

/** The switch's rows: the recent few under their own heading, then every repository. */
function switchEntries(
  repositories: readonly HelmRepositoryOption[],
  recent: readonly string[],
  current: string | undefined,
): DropdownMenuEntry[] {
  const row = (one: HelmRepositoryOption, id = one.id): DropdownMenuEntry => ({
    kind: "item",
    id,
    label: one.label,
    selected: one.id === current,
  });
  const led = recent.flatMap((id) => repositories.filter((one) => one.id === id));
  return [
    ...(led.length === 0
      ? []
      : [
          { kind: "label", id: "recent", label: "Recent" } as const,
          ...led.map((one) => row(one, `${RECENT}${one.id}`)),
          { kind: "separator", id: "recent-rule" } as const,
        ]),
    ...repositories.map((one) => row(one)),
  ];
}

/** The open Job chipped above the message box — `#1075`. */
export type HelmComposerChip = { jobHandle: string; title: string };

/**
 * What the Sessions box takes from outside, and Helm's takes the same. **No permission mode**: Helm
 * keeps its own rule. Everything absent is empty, and the box then offers only what it can.
 */
export type HelmBox = {
  /** The model and effort the next ask runs on. Absent is Auto. */
  model: string | null;
  effort: string | null;
  models: readonly string[];
  efforts: readonly string[];
  onTune: (tuning: { model: string | null; effort: string | null }) => void;
  /** What `/` offers. */
  commands: readonly { name: string; says: string }[];
  /** What `@` offers: other Sessions, Jobs, pull requests and branches. */
  taggable: readonly ComposerTag[];
  /** What `#` offers for the repository Helm answers for. */
  numbered: ComposerNumbered;
  /** Sketches drawn for this message and waiting to go. */
  drawn: readonly { id: string; title: string }[];
  onDraw: () => void;
  onRemoveDrawn: (id: string) => void;
};

/**
 * The composer under Helm's thread — #944. **Typing never answers a pending
 * question**; that is a card's own button, above this, in the dock's
 * questions zone. This only ever sends a fresh message.
 *
 * **The box is the Sessions box** (`SessionComposer`): ↵ sends and ⇧↵ breaks the line, and it
 * brings files and pictures, sketches, and `/`, `@` and `#`. What it keeps of Helm's own is
 * everything around it.
 */
export type HelmComposerProps = {
  /** The repository Helm answers for right now, by its Manifest id. Absent where nothing is servable yet. */
  current?: string;
  /**
   * Every repository the switch may choose. **Pointed at one of them, fewer
   * than two draws no control** — there is nothing to switch to. Pointed at
   * nothing, one is enough: it is somewhere to go. None is still nothing.
   */
  repositories?: HelmRepositoryOption[];
  /**
   * The few repositories picked last, by Manifest id, newest first — led with under a *Recent*
   * heading above the full list. The dock decides whether the list is long enough to want it and
   * passes none otherwise. An id naming no repository in `repositories` is left out.
   */
  recent?: readonly string[];
  /** The dock's own switch, on All repositories. The rail's pick never moves for it. */
  onSwitch?: (manifestId: string) => void;
  /**
   * Puts the session record on the clipboard in one press — `#1367`. **The
   * banner form of the error treatment**: the dock is a standing surface, and
   * the moment this exists for is a bad answer somebody wants to carry now.
   * That is why it is the split button's face and *Details* is behind the
   * caret. Absent draws no control.
   */
  onCopyRecord?: () => void;
  /**
   * Opens the same record to read. **The banner's *Details*, in that word** —
   * the sheet it opens is titled *Session record*, and that sheet carries a
   * copy of its own, so nothing is lost by it sitting under the caret. Absent
   * draws no control.
   */
  onOpenRecord?: () => void;
  /** The open Job, while its chip stands. Absent off a Job, or once its `×` has been pressed. */
  chip?: HelmComposerChip;
  /** The chip's own `×`. Omitted with no `chip` draws nothing to remove. */
  onRemoveChip?: () => void;
  /**
   * The footer's one sentence — where the person is, in the caller's own
   * words. **Built off the same context #1075 sends with every ask**, never a
   * second reading of the screen: a footer that could disagree with what
   * Helm was actually told would be worse than none. No lead-in — never
   * "Helm reads where you are: …". `#1094`. Absent draws no footer.
   */
  location?: string;
  /** The box's parts. Absent draws the box with nothing to offer beyond what is typed. */
  box?: Partial<HelmBox>;
  /** The message, with what went with it. **The box clears itself**, and a message the host refuses is not given back. */
  onSend: (sent: SentFromComposer) => void;
  /** Helm is working a turn. With `onStop`, a Stop sits beside Send. */
  working?: boolean;
  /** Ends the turn Helm is working. */
  onStop?: () => void;
  /** Nothing is servable, or the connection is down. */
  disabled?: boolean;
};

const NO_TAGS: readonly ComposerTag[] = [];
const NO_NAMES: readonly string[] = [];
const NO_COMMANDS: readonly { name: string; says: string }[] = [];
const NO_DRAWN: readonly { id: string; title: string }[] = [];
const NOTHING_TO_DO = (): void => undefined;

export function HelmComposer({
  current,
  repositories = [],
  recent = [],
  onSwitch,
  onCopyRecord,
  onOpenRecord,
  chip,
  onRemoveChip,
  location,
  box = {},
  onSend,
  working = false,
  onStop,
  disabled = false,
}: HelmComposerProps) {
  const label = repositories.find((one) => one.id === current)?.label;
  /**
   * Somewhere to point Helm that it is not pointed at already. **Pointed at a
   * repository, one repository is nothing to switch between**, which is all
   * this used to say. Pointed at nothing, that one repository is somewhere to
   * go, so the switch is drawn to pick it with. With none set up there is
   * nothing to switch to, and the line below says so.
   */
  const somewhere = current === undefined ? repositories.length > 0 : repositories.length > 1;
  const switching = onSwitch !== undefined && somewhere;
  /**
   * What Helm is pointed at, said once on this line. Pointed at nothing with
   * the switch drawn, the switch's own entry is what says it — and says what to
   * do about it — so this stays empty rather than repeating it: at the dock's
   * width the two together cut each other down to "No repo…" and "Choose a
   * rep…". With no switch to draw there is nowhere else for it to be said.
   */
  const naming = label ?? (switching ? undefined : "No repository to ask yet");

  return (
    <div className="armada-helm-composer">
      {chip === undefined ? null : (
        <AttachmentChip filename={`Job ${chip.jobHandle} · ${chip.title}`} onRemove={onRemoveChip} />
      )}
      <div className="armada-helm-composer__head">
        {naming === undefined ? null : (
          <span className="armada-helm-composer__repository">{naming}</span>
        )}
        {onSwitch === undefined || !switching ? null : (
          <DropdownMenu
            align="start"
            // In a layer of its own: the dock clips what hangs past its head row.
            portal
            // Named for what it does from where Helm is standing. *A different
            // repository* is a lie while it is pointed at none, and since this
            // change that is the switch's commonest moment — every unpointed
            // dock with anything set up draws it, one repository included.
            triggerAriaLabel={current === undefined ? "Point Helm at a repository" : "Point Helm at a different repository"}
            // Pointed at nothing it stands at an entry of its own, the same words the repository
            // asks elsewhere draw (`AskRepository`), and never at whichever repository is listed
            // first: the switch disagreeing with the chip beside it was the defect a native
            // `<select>` had, since a value matching no `<option>` displays the first one.
            triggerLabel={repositories.find((one) => one.id === current)?.label ?? "Choose a repository"}
            searchLabel="Find a repository"
            entries={switchEntries(repositories, recent, current)}
            onSelect={(picked) => onSwitch(picked.startsWith(RECENT) ? picked.slice(RECENT.length) : picked)}
          />
        )}
        {/* The record's two acts as one control — the owner's note of
            18 Sep 2026: three controls beside the repository wrapped this row
            onto a second line at the dock's own width. *Copy debug info*
            takes the face because it is the act this pair exists for — a bad
            answer somebody wants to carry now — and the reading it hides is
            still one press away behind the caret, in a sheet that carries a
            copy of its own.

            **Both handlers or nothing to split.** With one, there is no menu
            to put behind a caret, and a caret over an empty menu is a control
            that does not answer — so that case draws the plain button it
            already was, in the same weight as the split one. */}
        {onCopyRecord !== undefined && onOpenRecord !== undefined ? (
          <SplitButton
            variant="secondary"
            ground="card"
            size="sm"
            items={[{ label: "Details", onSelect: onOpenRecord }]}
            menuLabel="More ways to report this answer"
            onAction={onCopyRecord}
          >
            {COPY_DEBUG_INFO}
          </SplitButton>
        ) : onCopyRecord !== undefined ? (
          <Button variant="secondary" ground="card" size="sm" onClick={onCopyRecord}>
            {COPY_DEBUG_INFO}
          </Button>
        ) : onOpenRecord !== undefined ? (
          <Button variant="secondary" ground="card" size="sm" onClick={onOpenRecord}>
            Details
          </Button>
        ) : null}
      </div>
      {/* The Sessions box, whole: its row under the typing area holds the model and effort,
          the paperclip and the pencil, what waits to go and Send. Send is tinted Helm and not
          the accent, since it is not the view's one Primary (design-system.md, Component →
          token mapping → Helm dock). The mode is left off. */}
      <SessionComposer
        working={working}
        {...(onStop === undefined ? {} : { onStop })}
        modeHidden
        compact
        label="Ask Helm"
        disabled={disabled}
        send="secondary"
        slim
        grows="scroller"
        model={box.model ?? null}
        effort={box.effort ?? null}
        models={box.models ?? NO_NAMES}
        efforts={box.efforts ?? NO_NAMES}
        onTune={box.onTune ?? NOTHING_TO_DO}
        commands={box.commands ?? NO_COMMANDS}
        taggable={box.taggable ?? NO_TAGS}
        {...(box.numbered === undefined ? {} : { numbered: box.numbered })}
        tags={NO_TAGS}
        onTags={NOTHING_TO_DO}
        drawn={box.drawn ?? NO_DRAWN}
        onDraw={box.onDraw ?? NOTHING_TO_DO}
        onRemoveDrawn={box.onRemoveDrawn ?? NOTHING_TO_DO}
        // Kept per repository, and focused when the dock opens and when Helm is pointed elsewhere.
        draftKey={`helm:${current}`}
        focusFor={`helm:${current}`}
        onSend={onSend}
      />
      {/* Where the person is stays under the field, not inside it: it is a
          sentence to read, and the dock has no width to put it beside Send. */}
      {location === undefined ? null : (
        <span className="armada-helm-composer__location">{location}</span>
      )}
    </div>
  );
}
