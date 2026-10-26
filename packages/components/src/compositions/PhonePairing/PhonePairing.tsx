import { encode } from "uqr";
import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Stamp, StampSurface } from "../../primitives/Stamp/Stamp";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type PairedPhone = {
  id: string;
  name: string;
  /** When Fleet's Gateway last heard from it, in milliseconds since the epoch. */
  seenAt: number;
};

/** Why there is nothing to pair with. Absent where the Gateway answers. */
export type PhoneProblem =
  | { kind: "not_running" }
  /** Fleet's or the Gateway's own sentence, shown as it arrived. */
  | { kind: "said"; said: string };

/**
 * Settings → Phone: the Phone access switch, then pair, confirm and unpair.
 *
 * **Off draws the switch and its one line, nothing else.** Fleet runs the Gateway, so the switch is
 * the whole of starting it. A problem shows in place of pairing, and with the switch off only where a
 * press was refused.
 *
 * **The code is the whole of what a phone needs**, so the QR is drawn from `url` and the url is
 * printed under it for a phone that cannot scan. A claim shows once a phone has posted the code,
 * and Confirm is the only press that lets it in. Nothing is drawn for an empty slot: no phones
 * paired draws no list, no code drawn draws no square.
 */
export type PhonePairingProps = {
  /** Fleet's switch, `turning` while a press waits on it. Absent until Fleet has said, which draws no switch. */
  access?: { on: boolean; turning?: boolean };
  onTurn: (on: boolean) => void;
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

export function PhonePairing(props: PhonePairingProps) {
  const { access, onTurn, problem } = props;
  return (
    <div className="armada-phone">
      {access === undefined ? null : (
        <Switch
          checked={access.on}
          disabled={access.turning === true}
          onChange={(event) => onTurn(event.currentTarget.checked)}
          description="Paired phones reach this Mac through the tunnel you point at it while this is on, with Bridge open or closed."
        >
          Phone access
        </Switch>
      )}
      {problem === undefined ? null : (
        <Alert tone="neutral">{problem.kind === "not_running" ? "The Gateway is not running." : problem.said}</Alert>
      )}
      {problem !== undefined || access?.on !== true ? null : <Pairing {...props} />}
    </div>
  );
}

/** The code, the claim and the paired phones: what access on draws while the Gateway answers. */
function Pairing({ url, secondsLeft, claim, phones, onPair, onConfirm, onUnpair, onCopied }: PhonePairingProps) {
  return (
    <>
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
        <StampSurface>
          <ul className="armada-phone__list">
            {phones.map((phone) => (
              <li className="armada-phone__row" key={phone.id}>
                <p className="armada-phone__name">{phone.name}</p>
                <Stamp first="age" at={phone.seenAt} className="armada-phone__seen" />
                <Button variant="ghost" size="sm" aria-label={`Unpair ${phone.name}`} onClick={() => onUnpair(phone.id)}>
                  Unpair
                </Button>
              </li>
            ))}
          </ul>
        </StampSurface>
      )}
    </>
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
