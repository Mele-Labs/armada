// Create mod: a Mod Job started by describing what he wants, pointed at nothing. The palette's row and the Mods page's button both ask
// for the box with a window event, and the host drawn once beside the palette opens it. The box is the Dialog the Report and Release
// boxes use: one text field that keeps its draft across leaving it, and Start, which ↵ presses (⌘↵ as well, Shift+↵ makes a new line).

import { useEffect, useRef, useState } from "react";
import { Dialog, forgetKept, Textarea } from "@armada/components";

import { startModJobFrom } from "./mod-job";
import { OverTheApp } from "./session-view/OverTheApp";

const CREATE_MOD = "armada:create-mod";
const KEEP = "create-mod";

export function askToCreateMod(): void {
  window.dispatchEvent(new Event(CREATE_MOD));
}

/** The box, open while asked for. Esc or Cancel puts it away and keeps the words; a start that went clears them. */
export function CreateModHost() {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    // Opened after the key that chose the palette's row has finished: the Dialog listens on the window for ↵, and a kept draft would take that one as Start.
    let later: number | undefined;
    const asked = (): void => {
      later = window.setTimeout(() => setOpen(true), 0);
    };
    window.addEventListener(CREATE_MOD, asked);
    return () => {
      window.removeEventListener(CREATE_MOD, asked);
      window.clearTimeout(later);
    };
  }, []);
  return open ? <CreateModBox onClose={() => setOpen(false)} /> : null;
}

function CreateModBox({ onClose }: { onClose: () => void }) {
  const [said, setSaid] = useState("");
  const [starting, setStarting] = useState(false);
  const body = useRef<HTMLDivElement>(null);
  // The Dialog puts focus on Cancel as it opens; the words are what he came to type.
  useEffect(() => {
    const focus = window.setTimeout(() => body.current?.querySelector("textarea")?.focus(), 0);
    return () => window.clearTimeout(focus);
  }, []);
  const blank = said.trim() === "";
  const start = async (): Promise<void> => {
    if (blank || starting) return;
    setStarting(true);
    const started = await startModJobFrom(said);
    setStarting(false);
    if (!started) return;
    forgetKept(KEEP);
    onClose();
  };
  return (
    <OverTheApp open>
      <Dialog open tone="neutral" title="Create mod" confirmLabel="Start" confirmDisabled={blank || starting} onConfirm={() => void start()} onCancel={onClose}>
        <div ref={body}>
          <Textarea
            aria-label="Description"
            keep={KEEP}
            rows={4}
            value={said}
            onChange={(event) => setSaid(event.target.value)}
            onKeyDown={(event) => {
              // The Dialog sends on any ↵; a new line is Shift+↵, which it must not hear.
              if (event.key === "Enter" && event.shiftKey) event.nativeEvent.stopPropagation();
            }}
          />
        </div>
      </Dialog>
    </OverTheApp>
  );
}
