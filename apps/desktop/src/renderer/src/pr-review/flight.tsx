// What every act on the review that spends asks first, and the sheet that adds a tool. The tools are definitions from three places, as
// workflows are: Armada's, his Kit's (`~/.armada/review-tools/`), or the repository's (`.armada/review-tools/`). Each opens in a tab of
// the pull request's panes (`panes.tsx`), and Add a tool is the last entry of their + menu.

import { useEffect, useRef, useState } from "react";
import {
  Button,
  Checkbox,
  Dialog,
  Input,
  Radio,
  RadioGroup,
  Sheet,
  Textarea,
  Tooltip,
  forgetKept,
} from "@armada/components";
import { useAtFloor } from "@armada/shell";

import type { ReviewPr, ToolDef } from "./model";
import { useReviewStore, useReviews } from "./store";

/** Asks before an agent runs, since it spends tokens. **One box for all of it**, so every act that spends behaves the same. */
export function SpendConfirm() {
  const store = useReviewStore()!;
  const { spending } = useReviews();
  return (
    <SpendDialog
      spending={spending}
      onConfirm={(again) => store.confirmSpend(again)}
      onCancel={() => store.cancelSpend()}
    />
  );
}

/** The box itself, for any surface that spends: `spending` names the act while it asks, and `again` is Don't ask again. */
export function SpendDialog({
  spending,
  onConfirm,
  onCancel,
}: {
  spending: string | undefined;
  onConfirm: (again: boolean) => void;
  onCancel: () => void;
}) {
  const [again, setAgain] = useState(false);
  // Read through a ref, so a host that passes a new function each draw does not put the focus back on the box.
  const confirm = useRef(onConfirm);
  confirm.current = onConfirm;
  // Focus lands on the box in it. Enter runs from anywhere in it but Cancel, and Space turns the question off where focus is not on a button.
  useEffect(() => {
    if (spending === undefined) return;
    const frame = window.requestAnimationFrame(() =>
      document.querySelector<HTMLElement>('.armada-dialog input[type="checkbox"]')?.focus(),
    );
    const listen = (event: KeyboardEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      if (target === null || target.closest(".armada-dialog") === null) return;
      const onButton = target.closest("button") !== null;
      // In a walk window his Enter is the card's Next; the walk's own Enter on the dialog is the one that runs it.
      if (event.isTrusted && window.__armadaWalk !== undefined && event.key === "Enter") return;
      if (
        event.key === "Enter" &&
        !(onButton && target.closest("button")?.textContent?.startsWith("Cancel"))
      ) {
        event.preventDefault();
        event.stopPropagation();
        const skip = again;
        setAgain(false);
        confirm.current(skip);
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
  }, [
    spending,
    again,
  ]);
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
        onConfirm(skip);
      }}
      onCancel={() => {
        setAgain(false);
        onCancel();
      }}
    >
      <p>Running this uses tokens.</p>
    </Dialog>
  );
}

/** The sheet that adds a tool or an explanation: Fleet writes the file, in the Kit's folder or the repository's. */
export function AddTool({
  open,
  pr,
  onClose,
}: {
  open: boolean;
  pr: ReviewPr | undefined;
  onClose: () => void;
}) {
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
    void store
      .addTool({
        kind,
        name,
        checks,
        scope,
        manifestId: pr?.manifestId ?? "",
      })
      .then((said) => {
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
    <Sheet
      kind="review-tool"
      open={open}
      floating
      floor={floor}
      title="Add a tool"
      closeLabel="Close"
      closeBinding="Esc"
      onClose={close}
    >
      <form
        className="pr-add"
        aria-label="New tool"
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        <RadioGroup label="Kind">
          <Tooltip label="Finds things on a pull request, in a tab of its own" asChild>
            <span className="pr-add__choice">
              <Radio name="kind" checked={kind === "tool"} onChange={() => setKind("tool")}>
                Tool
              </Radio>
            </span>
          </Tooltip>
          <Tooltip label="Tells him one thing about a pull request, in a tab of its own" asChild>
            <span className="pr-add__choice">
              <Radio
                name="kind"
                checked={kind === "explanation"}
                onChange={() => setKind("explanation")}
              >
                Explanation
              </Radio>
            </span>
          </Tooltip>
        </RadioGroup>
        <Input label="Name" value={name} onChange={(event) => setName(event.target.value)} />
        <Textarea
          rows={6}
          keep={keep}
          aria-label={kind === "tool" ? "What it checks" : "What it explains"}
          placeholder={kind === "tool" ? "What it checks" : "What it explains"}
          value={checks}
          onChange={(event) => setChecks(event.target.value)}
        />
        <RadioGroup label="Save to">
          <Tooltip label="~/.armada/review-tools/, every repository on this machine" asChild>
            <span className="pr-add__choice">
              <Radio name="scope" checked={scope === "kit"} onChange={() => setScope("kit")}>
                Kit
              </Radio>
            </span>
          </Tooltip>
          <Tooltip
            label={`.armada/review-tools/ in ${pr?.repository ?? "the repository"}, shared with everyone on it`}
            asChild
          >
            <span className="pr-add__choice">
              <Radio
                name="scope"
                checked={scope === "repository"}
                onChange={() => setScope("repository")}
              >
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
