import { Check, CircleDashed } from "lucide-react";
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

/**
 * How far this Mac is along the way in a phone reaches it by. `named` is an address the owner wrote in
 * `pocket-address` for a tunnel of his own, and then the Tailscale steps are not his to take.
 */
export type PhoneWayIn = {
  tailscale: "missing" | "signed_out" | "running";
  /** What stands in front of the Gateway: nothing, `tailscale serve` (phones need Tailscale), or Funnel. */
  served: "none" | "tailnet" | "funnel";
  named: boolean;
  address?: string;
  /** A press of Turn on Funnel: waiting on it, the page the tailnet needs it allowed on, or Tailscale's refusal. */
  funnel?: { state: "turning" } | { state: "approve"; page: string } | { state: "said"; said: string };
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
 *
 * **The way in comes first.** With access on, the steps a phone needs before it can reach this Mac are
 * a row each, done or not, and only the first one not done carries its press. Pair a phone waits until
 * something stands in front of the Gateway. Once Funnel is on and a phone is paired, the steps fold to
 * one line.
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
  /** The way in, as the Gateway says it. Absent until it has, which draws no steps. */
  wayIn?: PhoneWayIn;
  onFunnel?: () => void;
  /** Open a page in the browser: Tailscale's download, or the tailnet's page to allow Funnel. */
  onOpen?: (page: string) => void;
};

const TAILSCALE_DOWNLOAD = "https://tailscale.com/download/mac";

/** The figure for time left: `4:32`. */
export function timeLeft(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

export function PhonePairing(props: PhonePairingProps) {
  const { access, onTurn, problem, wayIn } = props;
  const reachable = wayIn === undefined || wayIn.named || wayIn.served !== "none";
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
      {problem !== undefined || access?.on !== true ? null : (
        <>
          {wayIn === undefined ? null : <WayIn {...props} wayIn={wayIn} />}
          {reachable ? <Pairing {...props} /> : null}
        </>
      )}
    </div>
  );
}

type StepId = "install" | "sign_in" | "funnel" | "phone";

/** The steps to Funnel on, each with whether it is done and the one line that says how. */
function stepsOf(wayIn: PhoneWayIn, paired: boolean): readonly { id: StepId; name: string; why: string; done: boolean }[] {
  const where = wayIn.address ?? "this Mac's tailnet address";
  return [
    { id: "install", name: "Install Tailscale on this Mac", why: "Tailscale gives this Mac an https address.", done: wayIn.tailscale !== "missing" },
    { id: "sign_in", name: "Sign in to Tailscale", why: "From its icon in the menu bar.", done: wayIn.tailscale === "running" },
    {
      id: "funnel",
      name: "Turn on Funnel",
      why:
        wayIn.served === "tailnet"
          ? "Phones reach this Mac now only with Tailscale on them. With Funnel, any phone reaches it with nothing installed."
          : "Funnel puts this Mac's phone app on the internet at its tailnet address. Only a phone you paired here can use it.",
      done: wayIn.served === "funnel",
    },
    { id: "phone", name: "Add Armada to the iPhone", why: `In Safari on the iPhone, open ${where}, then Share, Add to Home Screen.`, done: paired },
  ];
}

/** The way in: one row a step, or one line once it is all done or the owner brought his own. */
function WayIn({ wayIn, phones, onFunnel, onOpen, onCopied }: PhonePairingProps & { wayIn: PhoneWayIn }) {
  if (wayIn.named)
    return (
      <p className="armada-phone__way">
        Phones reach this Mac at <span className="armada-phone__address">{wayIn.address}</span>, the address pocket-address names.
      </p>
    );
  const steps = stepsOf(wayIn, phones.length > 0);
  if (steps.every((step) => step.done))
    return (
      <p className="armada-phone__way">
        Phones reach this Mac at <span className="armada-phone__address">{wayIn.address}</span> through Tailscale Funnel.
      </p>
    );
  const current = steps.find((step) => !step.done)?.id;
  const funnel = wayIn.funnel;
  return (
    <section className="armada-phone__steps" aria-label="Way in">
      <p className="armada-phone__steps-title">Set up the way in</p>
      <ol className="armada-phone__list">
        {steps.map((step) => (
          <li key={step.id} className="armada-phone__step" aria-label={step.name} data-state={step.done ? "done" : step.id === current ? "current" : "waiting"}>
            {step.done ? (
              <Tooltip beside label="Done">
                <span className="armada-phone__done" role="img" aria-label="Done">
                  <Check size={16} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
            ) : (
              <Tooltip beside label="Not done yet">
                <span className="armada-phone__unset" role="img" aria-label="Not done yet">
                  <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
            )}
            <div className="armada-phone__step-text">
              <span className="armada-phone__name">{step.name}</span>
              {step.done || step.id !== current ? null : (
                <span className="armada-phone__why">
                  {step.id === "funnel" && funnel?.state === "approve"
                    ? "Your tailnet has not allowed Funnel for this Mac. Allow it on Tailscale's page, and this step finishes by itself."
                    : step.why}
                </span>
              )}
              {step.id === "funnel" && step.id === current && funnel?.state === "said" ? <Alert tone="neutral">{funnel.said}</Alert> : null}
            </div>
            <span className="armada-phone__act">
              {step.id !== current ? null : step.id === "install" ? (
                <Button variant="secondary" size="sm" onClick={() => onOpen?.(TAILSCALE_DOWNLOAD)}>
                  Get Tailscale
                </Button>
              ) : step.id === "funnel" && funnel?.state === "approve" ? (
                <Button variant="primary" size="sm" onClick={() => onOpen?.(funnel.page)}>
                  Allow Funnel
                </Button>
              ) : step.id === "funnel" ? (
                <Button variant="primary" size="sm" disabled={funnel?.state === "turning"} onClick={() => onFunnel?.()}>
                  Turn on Funnel
                </Button>
              ) : step.id === "phone" && wayIn.address !== undefined ? (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    const address = wayIn.address!;
                    void navigator.clipboard.writeText(address).then(
                      () => onCopied?.("The address"),
                      () => onCopied?.("The address"),
                    );
                  }}
                >
                  Copy address
                </Button>
              ) : null}
            </span>
          </li>
        ))}
      </ol>
    </section>
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
