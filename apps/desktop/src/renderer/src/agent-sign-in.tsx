// The agent's sign-in lapsing for the whole Fleet, and the one press that signs it in again. Behind a source the
// way Downshift is, so a Bridge with none draws nothing. The mock has its own (`mock/agent-sign-in.ts`).

import { createContext, useContext, useSyncExternalStore } from "react";
import { LoaderCircle } from "lucide-react";
import { Toast, Tooltip } from "@armada/components";

import "./agent-sign-in.css";

/** Where the sign-in stands. `signed_out_since` absent: signed in. */
export type AgentSignIn = { signed_out_since?: string; signing_in: boolean };

export type AgentSignInSource = {
  get(): AgentSignIn;
  subscribe(on: () => void): () => void;
  /** Start the sign-in. */
  signIn(): void;
};

const AgentSignInContext = createContext<AgentSignInSource | null>(null);
export const AgentSignInSourceProvider = AgentSignInContext.Provider;

const NO_STATE: AgentSignIn = { signing_in: false };
const NONE = { get: () => NO_STATE, subscribe: () => () => undefined };

/** Stays until the sign-in is settled. One act, which goes while the sign-in runs. */
export function AgentSignInAlert() {
  const source = useContext(AgentSignInContext);
  const state = useSyncExternalStore((source ?? NONE).subscribe, (source ?? NONE).get);
  if (source === null || state.signed_out_since === undefined) return null;
  if (state.signing_in) {
    return (
      <Toast>
        <span className="armada-agent-sign-in__title">
          Claude is signed out
          <Tooltip label="Signing in">
            <span className="armada-agent-sign-in__ring" role="img" aria-label="Signing in">
              <LoaderCircle size={12} aria-hidden />
            </span>
          </Tooltip>
        </span>
      </Toast>
    );
  }
  return (
    <Toast actionLabel="Sign in" onAction={source.signIn}>
      Claude is signed out
    </Toast>
  );
}
