// The first thing on Pair: his Macs, listed by the store his GitHub account signs in to. Signed out it is one button, mid
// sign-in it is GitHub's code, signed in it is a row per Mac with a Pair button. A press on it pairs with no code and no Confirm,
// because the store has already shown the Mac that this phone is his. The pairing link below is the other way.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { CircleCheck, CircleDashed, RefreshCw, ShieldX, TriangleAlert, Unplug } from "lucide-react";
import { Button } from "./kit/components/primitives/Button/Button";
import { Skeleton } from "./kit/components/primitives/Skeleton/Skeleton";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";

import { Refused } from "./client";
import { useFleet } from "./fleet";
import { forgetStoreSignIn, keepStoreSignIn, storeSignIn, type StoreSignIn } from "./machines";
import { generateKey, keepPaired } from "./pairing";
import {
  macsOf,
  pollSignIn,
  startSignIn,
  storeOf,
  vouch,
  type DeviceCode,
  type Peer,
} from "./vouched";
import { IconButton, Trouble } from "./ui";

type Stage =
  | {
      at: "loading";
    }
  | {
      at: "unavailable";
    }
  | {
      at: "out";
      store: string;
      clientId: string;
      trouble: string;
    }
  | {
      at: "code";
      store: string;
      clientId: string;
      code: DeviceCode;
      key: CryptoKeyPair;
    }
  | {
      at: "in";
      sign: StoreSignIn;
    };

/** What a row's last Pair press came to; a row with none is ready. */
type Act =
  | {
      at: "pairing";
    }
  | {
      at: "refused";
      status: number;
      says: string;
    };

const sleep = (seconds: number, signal: AbortSignal) =>
  new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, seconds * 1000);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        resolve();
      },
      {
        once: true,
      },
    );
  });

const bucket = {
  ready: "idle",
  pairing: "working",
  paired: "done",
  refused: "failed",
} as const;

/** A row's state as a glyph in its hue, named by its tooltip. */
function Mark({ state, refused }: { state: keyof typeof bucket; refused?: number }) {
  const [Glyph, said] =
    state === "paired"
      ? [
          CircleCheck,
          "Paired",
        ]
      : state === "pairing"
        ? [
            CircleDashed,
            "Pairing",
          ]
        : state === "ready"
          ? [
              CircleDashed,
              "Not paired",
            ]
          : refused === 403
            ? [
                ShieldX,
                "Not found in your store",
              ]
            : refused === 0
              ? [
                  Unplug,
                  "Not answering",
                ]
              : [
                  TriangleAlert,
                  "Could not pair",
                ];
  return (
    <Tooltip label={said}>
      <span
        className={state === "pairing" ? "pk-glyph pk-ring pk-ring--dot" : "pk-glyph"}
        data-bucket={bucket[state]}
        role="img"
        aria-label={said}
      >
        {state === "pairing" ? null : <Glyph size={16} aria-hidden="true" />}
      </span>
    </Tooltip>
  );
}

function Row({
  peer,
  paired,
  act,
  unnamed,
  onPair,
}: {
  peer: Peer;
  paired: boolean;
  act: Act | undefined;
  unnamed: boolean;
  onPair: () => void;
}) {
  const state = paired ? "paired" : (act?.at ?? "ready");
  return (
    <li className="pk-job">
      <div
        className="pk-job__card pk-macs__row"
        data-lamp={
          state === "paired"
            ? "done"
            : state === "pairing"
              ? "running"
              : state === "refused"
                ? "failed"
                : undefined
        }
      >
        <Mark
          state={state}
          {...(act?.at === "refused"
            ? {
                refused: act.status,
              }
            : {})}
        />
        <span className="pk-job__title">{peer.machine}</span>
        <Button
          variant="secondary"
          size="sm"
          ground="sunken"
          aria-label={
            paired
              ? `${peer.machine} paired`
              : state === "refused"
                ? `Try again with ${peer.machine}`
                : `Pair ${peer.machine}`
          }
          disabled={paired || unnamed}
          pending={state === "pairing"}
          onClick={onPair}
        >
          {paired ? "Paired" : state === "refused" ? "Try again" : "Pair"}
        </Button>
        {!paired && act?.at === "refused" ? (
          <span className="pk-job__says pk-macs__says">{act.says}</span>
        ) : null}
      </div>
    </li>
  );
}

export function YourMacs({ name }: { name: string }) {
  const fleet = useFleet();
  const here = window.location.origin;
  const [stage, setStage] = useState<Stage>({
    at: "loading",
  });
  const [starting, setStarting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [peers, setPeers] = useState<Peer[] | undefined>(undefined);
  const [trouble, setTrouble] = useState("");
  const [again, setAgain] = useState(0);
  const [acts, setActs] = useState<Record<string, Act>>({});
  // The poll reads the name as it is when it goes out, not as it was when the sign in began.
  const named = useRef(name);
  named.current = name;

  /** Back to the signed-out button, with what went wrong; a Mac with no sign in set up has none to offer. */
  const signedOut = async (says: string) => {
    const found = await storeOf(here);
    setStage(
      found === undefined
        ? {
            at: "unavailable",
          }
        : {
            at: "out",
            ...found,
            trouble: says,
          },
    );
  };

  useEffect(() => {
    let live = true;
    void (async () => {
      const kept = await storeSignIn().catch(() => undefined);
      if (!live) return;
      if (kept !== undefined) {
        setStage({
          at: "in",
          sign: kept,
        });
        return;
      }
      const found = await storeOf(here);
      if (!live) return;
      setStage(
        found === undefined
          ? {
              at: "unavailable",
            }
          : {
              at: "out",
              ...found,
              trouble: "",
            },
      );
    })();
    return () => {
      live = false;
    };
  }, [
    here,
  ]);

  const code = stage.at === "code" ? stage : undefined;
  useEffect(() => {
    if (code === undefined) return;
    const stop = new AbortController();
    void (async () => {
      let wait = code.code.interval;
      while (!stop.signal.aborted) {
        await sleep(wait, stop.signal);
        if (stop.signal.aborted) return;
        try {
          const polled = await pollSignIn(
            code.store,
            code.clientId,
            code.code,
            code.key,
            named.current.trim() || "Phone",
          );
          if (stop.signal.aborted) return;
          if (polled.at === "in") {
            const sign = {
              store: code.store,
              deviceId: polled.deviceId,
              key: code.key.privateKey,
            };
            await keepStoreSignIn(sign);
            setStage({
              at: "in",
              sign,
            });
            return;
          }
          if (polled.interval !== undefined) wait = polled.interval;
        } catch (why) {
          // A store that does not answer once is asked again; GitHub's own answers end the sign in.
          if (why instanceof Refused && why.status === 0) continue;
          if (stop.signal.aborted) return;
          setStage({
            at: "out",
            store: code.store,
            clientId: code.clientId,
            trouble: why instanceof Refused ? why.message : "",
          });
          return;
        }
      }
    })();
    return () => stop.abort();
  }, [
    code,
  ]);

  const sign = stage.at === "in" ? stage.sign : undefined;
  useEffect(() => {
    if (sign === undefined) return;
    let live = true;
    setTrouble("");
    macsOf(sign).then(
      (found) => live && setPeers(found),
      (why: unknown) => {
        if (!live) return;
        if (why instanceof Refused && why.status === 401) {
          void forgetStoreSignIn().catch(() => undefined);
          setPeers(undefined);
          void signedOut(why.message);
          return;
        }
        setPeers([]);
        setTrouble(why instanceof Refused ? why.message : "");
      },
    );
    return () => {
      live = false;
    };
  }, [
    sign,
    again,
  ]);

  const begin = async () => {
    if (stage.at !== "out") return;
    const { store, clientId } = stage;
    setStarting(true);
    try {
      const key = await generateKey();
      setStage({
        at: "code",
        store,
        clientId,
        code: await startSignIn(store, clientId),
        key,
      });
    } catch (why) {
      setStage({
        ...stage,
        trouble: why instanceof Refused ? why.message : "",
      });
    } finally {
      setStarting(false);
    }
  };

  const copy = async () => {
    if (code === undefined) return;
    try {
      await navigator.clipboard.writeText(code.code.user_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // The code stays on the screen to be read and typed.
    }
  };

  const pair = async (peer: Peer) => {
    if (sign === undefined) return;
    setActs((held) => ({
      ...held,
      [peer.device]: {
        at: "pairing",
      },
    }));
    try {
      const machine = await vouch(sign, peer, named.current);
      await keepPaired(
        machine,
        fleet.macs.map((mac) => mac.machine),
      );
      setActs(({ [peer.device]: _done, ...rest }) => rest);
    } catch (why) {
      setActs((held) => ({
        ...held,
        [peer.device]: {
          at: "refused",
          status: why instanceof Refused ? why.status : 0,
          says: why instanceof Refused ? why.message : "",
        },
      }));
    }
  };

  let body: ReactNode = null;
  if (stage.at === "loading")
    body = (
      <div className="pk-macs__wait" role="status" aria-label="Loading">
        <Skeleton width="100%" />
      </div>
    );
  else if (stage.at === "unavailable")
    body = <Trouble says="Sign in with GitHub is not set up on this Mac." />;
  else if (stage.at === "out")
    body = (
      <>
        <Button
          variant="primary"
          className="pk-big"
          pending={starting}
          onClick={() => void begin()}
        >
          {stage.trouble === "" ? "Sign in with GitHub" : "Sign in again"}
        </Button>
        <Trouble says={stage.trouble} />
      </>
    );
  else if (stage.at === "code")
    body = (
      <div className="pk-signin">
        <p className="pk-signin__code">{stage.code.user_code}</p>
        <div className="pk-signin__acts">
          <Button variant="secondary" size="sm" ground="sunken" onClick={() => void copy()}>
            {copied ? "Copied" : "Copy"}
          </Button>
          <a
            className="armada-button"
            data-variant="secondary"
            data-size="sm"
            data-ground="sunken"
            href={stage.code.verification_uri}
            target="_blank"
            rel="noreferrer"
          >
            Open GitHub
          </a>
          <Button
            variant="ghost"
            size="sm"
            onClick={() =>
              setStage({
                at: "out",
                store: stage.store,
                clientId: stage.clientId,
                trouble: "",
              })
            }
          >
            Cancel
          </Button>
        </div>
        <p className="pk-signin__wait pk-ring pk-ring--line" role="status">
          Waiting for GitHub
        </p>
      </div>
    );
  else if (peers === undefined)
    body = (
      <ul className="pk-jobs pk-macs__wait" aria-label="Loading">
        <li>
          <Skeleton width="100%" />
        </li>
        <li>
          <Skeleton width="100%" />
        </li>
      </ul>
    );
  else
    body = (
      <>
        <ul className="pk-jobs">
          {peers.map((peer) => (
            <Row
              key={peer.device}
              peer={peer}
              paired={fleet.macs.some(
                (mac) => mac.machine.origin === peer.address.replace(/\/+$/, ""),
              )}
              act={acts[peer.device]}
              unnamed={name.trim() === ""}
              onPair={() => void pair(peer)}
            />
          ))}
        </ul>
        <Trouble says={trouble} />
      </>
    );

  return (
    <section className="pk-group pk-macs" aria-label="Your Macs">
      <div className="pk-macs__head">
        <h2 className="pk-group__head">Your Macs</h2>
        {stage.at === "in" ? (
          <IconButton label="Refresh Macs" onClick={() => setAgain((one) => one + 1)}>
            <RefreshCw size={16} aria-hidden="true" />
          </IconButton>
        ) : null}
      </div>
      {body}
    </section>
  );
}
