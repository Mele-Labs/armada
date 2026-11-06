import { Check, ChevronDown } from "lucide-react";
import { useEffect, useRef } from "react";
import type { KeyboardEvent } from "react";

import { Popover } from "../../primitives/Popover/Popover";
import { Slider } from "../../primitives/Slider/Slider";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type TunePickerProps = {
  model: string | null;
  effort: string | null;
  models: readonly string[];
  /** Lowest first. */
  efforts: readonly string[];
  /** The words a model is drawn as; `null` is the one the Session follows while it names none. */
  said: (model: string | null) => string;
  onTune: (tuning: { model: string | null; effort: string | null }) => void;
};

const AUTO = "";

/**
 * The model and the effort of the next turn in one control: the model's name and a meter of the effort on the button, and
 * pressed, the models to pick from over an effort slider. ↑↓ move between models, Tab reaches the slider, ←→ slide it, and
 * Enter or Esc put it away.
 */
export function TunePicker({ model, effort, models, efforts, said, onTune }: TunePickerProps) {
  const level = effort === null ? 0 : efforts.indexOf(effort) + 1;
  return (
    <Popover
      align="start"
      label="Model and effort"
      trigger={
        <Tooltip label="The model and effort the next turn runs on">
          <button type="button" className="armada-tune__trigger" aria-haspopup="dialog">
            <span className="armada-tune__model">{said(model)}</span>
            <span
              className="armada-tune__meter"
              role="img"
              aria-label={effort === null ? "Effort: auto" : `Effort: ${effort}`}
            >
              {efforts.map((one, at) => (
                <span key={one} className="armada-tune__bar" data-on={at < level || undefined} />
              ))}
            </span>
            <ChevronDown size={14} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      }
    >
      {(close) => (
        <TunePanel
          model={model}
          effort={effort}
          models={models}
          efforts={efforts}
          said={said}
          onTune={onTune}
          close={close}
        />
      )}
    </Popover>
  );
}

function TunePanel({
  model,
  effort,
  models,
  efforts,
  said,
  onTune,
  close,
}: TunePickerProps & { close: () => void }) {
  const list = useRef<HTMLDivElement>(null);
  const choices: (string | null)[] = [null, ...models];
  // Opened, the model in use has the focus, so the keys work at once.
  useEffect(() => {
    list.current?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
  }, []);
  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" && (event.target as HTMLInputElement).type === "range") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const rows = [...(list.current?.querySelectorAll<HTMLElement>('[role="radio"]') ?? [])];
    const at = rows.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    event.preventDefault();
    const next = rows[(at + (event.key === "ArrowDown" ? 1 : rows.length - 1)) % rows.length];
    next?.focus();
  };
  return (
    <div className="armada-tune" onKeyDown={onKey}>
      <div ref={list} className="armada-tune__models" role="radiogroup" aria-label="Model">
        {choices.map((one) => {
          const picked = one === model;
          return (
            <button
              key={one ?? AUTO}
              type="button"
              role="radio"
              aria-checked={picked}
              tabIndex={picked ? 0 : -1}
              className="armada-tune__row"
              onClick={() =>
                onTune({
                  model: one,
                  effort,
                })
              }
            >
              <span>{said(one)}</span>
              {picked ? <Check size={14} strokeWidth={2} aria-hidden /> : null}
            </button>
          );
        })}
      </div>
      <div className="armada-tune__effort">
        <span className="armada-tune__label">Effort</span>
        <Slider
          aria-label="Effort"
          steps={[AUTO, ...efforts]}
          value={effort ?? AUTO}
          show={(step) => (step === AUTO ? "Auto" : step)}
          onChange={(step) =>
            onTune({
              model,
              effort: step === AUTO ? null : step,
            })
          }
        />
      </div>
    </div>
  );
}
