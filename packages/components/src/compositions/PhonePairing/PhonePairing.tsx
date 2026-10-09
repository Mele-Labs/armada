import { encode } from "uqr";
import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type PairedPhone = {
  id: string;
  name: string;
  /** When Fleet's Gateway last heard from it, already worded: "2 minutes ago". */
  seen: string;
};

/** Why there is nothing to pair with. Absent where the Gateway answers. */
export type PhoneProblem =
  | { kind: "not_running" }
  /** The Gateway's own sentence, shown as it arrived. */
  | { kind: "said"; said: string };

/**
 * Settings → Phone: pair, confirm and unpair.
 *
 * **The code is the whole of what a phone needs**, so the QR is drawn from `url` and the url is
 * printed under it for a phone that cannot scan. A claim shows once a phone has posted the code,
 * and Confirm is the only press that lets it in. Nothing is drawn for an empty slot: no phones
 * paired draws no list, no code drawn draws no square.
 */
export type PhonePairingProps = {
  problem?: PhoneProblem;
  /** The pairing address with its one-time code. Absent until Pair a phone is pressed. */
  url?: string;
  /** What is left on the code. Names the figure beside the QR. */
  secondsLeft?: number;
  /** A phone that posted the code and is waiting on Confirm. */
  claim?: { device: string };
  phones: readonly PairedPhone[];
  onPair: () => void;
  onConfirm: () => void;
  onUnpair: (id: string) => void;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (value: string) => void;
};

/** The figure for time left: `4:32`. */
export function timeLeft(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

export function PhonePairing({ problem, url, secondsLeft, claim, phones, onPair, onConfirm, onUnpair, onCopied }: PhonePairingProps) {
  if (problem !== undefined) {
    return (
      <div className="armada-phone">
        <Alert tone="neutral" title={problem.kind === "not_running" ? "The Gateway is not running" : undefined}>
          {problem.kind === "not_running" ? <code className="armada-phone__command">armada pocket</code> : problem.said}
        </Alert>
      </div>
    );
  }
  return (
    <div className="armada-phone">
      {url === undefined ? (
        <div>
          <Button variant="primary" onClick={onPair}>
            Pair a phone
          </Button>
        </div>
      ) : (
        <div className="armada-phone__code">
          <div className="armada-phone__qr" role="img" aria-label="Pairing code">
            <Qr text={url} />
          </div>
          <div className="armada-phone__code-facts">
            <p className="armada-phone__url">{url}</p>
            <div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() =>
                  // A failed write is otherwise indistinguishable from a dead button, so the surface is told either way.
                  void navigator.clipboard.writeText(url).then(
                    () => onCopied?.("The link"),
                    () => onCopied?.("The link"),
                  )
                }
              >
                Copy link
              </Button>
            </div>
            {secondsLeft === undefined ? null : (
              <Tooltip label="Time left on this code">
                <span className="armada-phone__left" tabIndex={0}>
                  {timeLeft(secondsLeft)}
                </span>
              </Tooltip>
            )}
          </div>
        </div>
      )}

      {claim === undefined ? null : (
        <div className="armada-phone__claim" role="group" aria-label={`${claim.device} wants to pair`}>
          <p className="armada-phone__name">{claim.device} wants to pair</p>
          <Button variant="primary" onClick={onConfirm}>
            Confirm
          </Button>
        </div>
      )}

      {phones.length === 0 ? null : (
        <ul className="armada-phone__list">
          {phones.map((phone) => (
            <li className="armada-phone__row" key={phone.id}>
              <p className="armada-phone__name">{phone.name}</p>
              <Tooltip label="Last seen">
                <span className="armada-phone__seen" tabIndex={0}>
                  {phone.seen}
                </span>
              </Tooltip>
              <Button variant="ghost" size="sm" aria-label={`Unpair ${phone.name}`} onClick={() => onUnpair(phone.id)}>
                Unpair
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Modules of empty margin round the code; the spec asks four and phone cameras rely on it. */
const QUIET = 4;

/** The QR of `text`, one path of unit squares. Its colours are the stylesheet's: a camera needs dark on light in both themes. */
function Qr({ text }: { text: string }) {
  const { data: rows } = encode(text, { ecc: "M", border: 0 });
  const size = rows.length + QUIET * 2;
  const path = rows
    .flatMap((row, y) => row.map((dark, x) => (dark ? `M${x + QUIET} ${y + QUIET}h1v1h-1z` : "")))
    .join("");
  return (
    <svg className="armada-phone__qr-mark" viewBox={`0 0 ${size} ${size}`} shapeRendering="crispEdges" aria-hidden="true">
      <path d={path} />
    </svg>
  );
}
