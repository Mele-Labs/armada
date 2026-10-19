// The review's Flight control: one tile per review tool, keyed 1 to 9, in the Cockpit's own tile language. **A
// tool runs for the open pull request only when pressed**, because an agent run is expensive, and shows running on
// its tile. The tools are definitions from three places, as workflows are, and each tile carries its place as a
// small glyph: Armada's, his Kit's (`~/.armada/review-tools/`), or the repository's (`.armada/review-tools/`).
// The last tile adds one: Fleet writes the file, in the Kit's folder or the repository's.

import { useEffect, useState } from "react";
import { CircleX, History, LoaderCircle, RefreshCw, SquarePlus } from "lucide-react";
import { Button, Checkbox, Dialog, Input, Kbd, PrMark, Radio, RadioGroup, Sheet, Textarea, Tooltip, agoOf, forgetKept } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { ReviewKeys } from "./keys";
import { SOURCE_MARK, TOOL_MARK } from "./marks";
import type { ReviewPr, ToolDef } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { isWalk, onlyTools, toolsOf } from "./tools";
import { WalkSheets } from "./walk";

/** What a tile says under its glyph, where the tool's name is long. */
const SHORT: Record<string, string> = { "api-breaking-changes": "API breaks" };

const WHOSE = { armada: "Armada's", kit: "the Kit's", repository: "the repository's" } as const;

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/** The tiles, as a block for a Flight control panel to hold. */
export function ReviewTools() {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const pr = reviews.prs.find((one) => one.key === reviews.open);
  const work = pr === undefined ? NO_WORK : (reviews.work[pr.key] ?? NO_WORK);
  const tools = onlyTools(toolsOf(reviews, pr?.manifestId));
  const [adding, setAdding] = useState(false);
  const { sheet } = reviews;

  const press = (tool: ToolDef) => {
    if (pr === undefined) return;
    const kept = work.runs[tool.id];
    // The Walk is found a place before it is written, and one already looking, asking or failed to find shows as it stands.
    const finding = isWalk(tool) && work.walk.find.state !== "idle";
    // A kept result opens at once. Only a run that has not happened asks first.
    if (!tool.agent || finding || (kept !== undefined && kept.state !== "idle")) store.runTool(pr.key, tool);
    else if (isWalk(tool)) store.spend(tool.name, () => store.findPlace(pr.key));
    else store.spend(tool.name, () => store.runTool(pr.key, tool));
  };

  // The keys 1 to 9 press the tiles, where no box is being typed in.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey || typing(event.target) || adding || sheet !== undefined) return;
      const at = Number(event.key) - 1;
      if (!Number.isInteger(at) || at < 0 || at > 8) return;
      if (at < tools.length) press(tools[at]!);
      else if (at === tools.length) setAdding(true);
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  });

  const count = tools.length + 1;
  return (
    <>
      <div className="deck-controls__block pr-tools" data-n={count} style={{ "--n": count } as React.CSSProperties}>
        {tools.map((tool, at) => {
          const run = work.runs[tool.id];
          const running = run?.state === "running";
          const blocked = pr === undefined;
          const kept = run?.state === "done" && tool.agent ? run : undefined;
          // A result is kept for the commit it ran on. A newer commit marks it out of date, and it is still shown.
          const stale = kept !== undefined && pr !== undefined && kept.head !== pr.head;
          const mark = TOOL_MARK[tool.mark];
          const source = SOURCE_MARK[tool.source];
          return (
            <Tooltip key={tool.id} label={tool.name} shortcut={String(at + 1)} asChild>
              <button type="button" className="deck-tile pr-tool" aria-label={tool.name} aria-pressed={work.view === tool.id} aria-disabled={blocked || undefined} aria-busy={running || undefined} data-tool={tool.mark} data-source={tool.source} data-stale={stale || undefined} onClick={() => press(tool)}>
                {running ? <LoaderCircle size={16} aria-hidden="true" className="pr-tool__spin" /> : <mark.Glyph size={16} aria-hidden="true" />}
                <span className="deck-tile__text pr-tool__text">{SHORT[tool.id] ?? tool.name}</span>
                <Kbd className="deck-tile__key" aria-hidden="true">
                  {at + 1}
                </Kbd>
                {run?.state === "failed" && tool.agent ? (
                  <span className="pr-tool__kept">
                    <PrMark Glyph={CircleX} said={`Failed. ${run.said}`} tone="failed" />
                  </span>
                ) : null}
                {kept === undefined ? null : (
                  <span className="pr-tool__kept">
                    <PrMark
                      Glyph={stale ? RefreshCw : History}
                      said={stale ? `Out of date. Ran on ${kept.head.slice(0, 7)}, the pull request is at ${pr!.head.slice(0, 7)}` : `Kept. Ran ${agoOf(new Date(kept.at).toISOString())} on ${kept.head.slice(0, 7)}`}
                    />
                  </span>
                )}
                <span className="pr-tool__source" data-source={tool.source}>
                  <PrMark Glyph={source.Glyph} said={tool.replaces === undefined ? source.said : `${source.said}. Replaces ${WHOSE[tool.replaces]} ${tool.name}`} />
                </span>
              </button>
            </Tooltip>
          );
        })}
        <Tooltip label="Add a tool" shortcut={String(tools.length + 1)} asChild>
          <button type="button" className="deck-tile pr-tool" aria-label="Add a tool" onClick={() => setAdding(true)}>
            <SquarePlus size={16} aria-hidden="true" />
            <span className="deck-tile__text pr-tool__text">Add a tool</span>
            <Kbd className="deck-tile__key" aria-hidden="true">
              {tools.length + 1}
            </Kbd>
          </button>
        </Tooltip>
      </div>
      <AddTool open={adding} pr={pr} onClose={() => setAdding(false)} />
      <SpendConfirm />
      <WalkSheets />
      <ReviewKeys />
    </>
  );
}

/** Asks before an agent runs, since it spends tokens. **One box for all of it**, so every act that spends behaves the same. */
export function SpendConfirm() {
  const store = useReviewStore()!;
  const { spending } = useReviews();
  const [again, setAgain] = useState(false);
  // Focus lands on the box in it. Enter runs from anywhere in it but Cancel, and Space turns the question off where focus is not on a button.
  useEffect(() => {
    if (spending === undefined) return;
    const frame = window.requestAnimationFrame(() => document.querySelector<HTMLElement>('.armada-dialog input[type="checkbox"]')?.focus());
    const listen = (event: KeyboardEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      if (target === null || target.closest(".armada-dialog") === null) return;
      const onButton = target.closest("button") !== null;
      // In a walk window his Enter is the card's Next; the walk's own Enter on the dialog is the one that runs it.
      if (event.isTrusted && window.__armadaWalk !== undefined && event.key === "Enter") return;
      if (event.key === "Enter" && !(onButton && target.closest("button")?.textContent?.startsWith("Cancel"))) {
        event.preventDefault();
        event.stopPropagation();
        const skip = again;
        setAgain(false);
        store.confirmSpend(skip);
      } else if (event.key === " " && !onButton && target.tagName !== "INPUT") {
        event.preventDefault();
        setAgain((now) => !now);
      }
    };
    window.addEventListener("keydown", listen, true);
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("keydown", listen, true);
    };
  }, [spending, again, store]);
  return (
    <Dialog
      open={spending !== undefined}
      tone="neutral"
      title={spending ?? ""}
      confirmLabel="Run"
      field={
        <Checkbox checked={again} onChange={(event) => setAgain(event.target.checked)}>
          Don't ask again
        </Checkbox>
      }
      onConfirm={() => {
        const skip = again;
        setAgain(false);
        store.confirmSpend(skip);
      }}
      onCancel={() => {
        setAgain(false);
        store.cancelSpend();
      }}
    >
      <p>Running this uses tokens.</p>
    </Dialog>
  );
}

/** The Reviews page's panel: the same tiles under the same band as the Cockpit's Flight control, at the foot of the review surface. */
export function ReviewFlight() {
  return (
    <div className="deck-foot">
      <section className="deck-panel deck-controls" aria-label="Flight controls" tabIndex={-1}>
        <header className="deck-band">Flight control</header>
        <ReviewTools />
      </section>
    </div>
  );
}

function AddTool({ open, pr, onClose }: { open: boolean; pr: ReviewPr | undefined; onClose: () => void }) {
  const store = useReviewStore()!;
  const floor = useAtFloor();
  const [name, setName] = useState("");
  const [checks, setChecks] = useState("");
  const [scope, setScope] = useState<"kit" | "repository">("kit");
  const [kind, setKind] = useState<ToolDef["kind"]>("tool");
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const [saving, setSaving] = useState(false);
  const leftOut = useReviews().leftOut[pr?.manifestId ?? ""] ?? [];
  const ready = name.trim() !== "" && checks.trim() !== "" && !saving;
  const keep = "pr-review:add-tool:checks";
  const close = () => {
    setRefused(undefined);
    onClose();
  };
  const save = () => {
    if (!ready) return;
    setSaving(true);
    void store.addTool({ kind, name, checks, scope, manifestId: pr?.manifestId ?? "" }).then((said) => {
      setSaving(false);
      setRefused(said);
      if (said !== undefined) return;
      forgetKept(keep);
      setName("");
      setChecks("");
      setScope("kit");
      setKind("tool");
      onClose();
    });
  };
  return (
    <Sheet kind="review-tool" open={open} floating floor={floor} title="Add a tool" closeLabel="Close" closeBinding="Esc" onClose={close}>
      <form
        className="pr-add"
        aria-label="New tool"
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        <RadioGroup label="Kind">
          <Tooltip label="Finds things on a pull request, as a tile in the Flight control" asChild>
            <span className="pr-add__choice">
              <Radio name="kind" checked={kind === "tool"} onChange={() => setKind("tool")}>
                Tool
              </Radio>
            </span>
          </Tooltip>
          <Tooltip label="Tells him one thing about a pull request, chosen under its description" asChild>
            <span className="pr-add__choice">
              <Radio name="kind" checked={kind === "explanation"} onChange={() => setKind("explanation")}>
                Explanation
              </Radio>
            </span>
          </Tooltip>
        </RadioGroup>
        <Input label="Name" value={name} onChange={(event) => setName(event.target.value)} />
        <Textarea rows={6} keep={keep} aria-label={kind === "tool" ? "What it checks" : "What it explains"} placeholder={kind === "tool" ? "What it checks" : "What it explains"} value={checks} onChange={(event) => setChecks(event.target.value)} />
        <RadioGroup label="Save to">
          <Tooltip label="~/.armada/review-tools/, every repository on this machine" asChild>
            <span className="pr-add__choice">
              <Radio name="scope" checked={scope === "kit"} onChange={() => setScope("kit")}>
                Kit
              </Radio>
            </span>
          </Tooltip>
          <Tooltip label={`.armada/review-tools/ in ${pr?.repository ?? "the repository"}, shared with everyone on it`} asChild>
            <span className="pr-add__choice">
              <Radio name="scope" checked={scope === "repository"} onChange={() => setScope("repository")}>
                Repository
              </Radio>
            </span>
          </Tooltip>
        </RadioGroup>
        <div className="pr-add__bar">
          {refused === undefined ? null : (
            <p className="pr-add__said" role="alert">
              {refused}
            </p>
          )}
          <Button variant="ghost" onClick={close}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" disabled={!ready}>
            Save
          </Button>
        </div>
      </form>
      {leftOut.length === 0 ? null : (
        <ul className="pr-add__left" aria-label="Left out">
          {leftOut.map((one) => (
            <li key={one.file} className="pr-add__left-row">
              <span className="pr-add__left-file mono">{one.file}</span>
              {one.said}
            </li>
          ))}
        </ul>
      )}
    </Sheet>
  );
}
