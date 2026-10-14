// The account on the wire, mirrored by hand from `crates/ipc/src/accounts.rs`.

/** One profile this machine has joined. `account_id` is the forge's numeric id and names the profile in every act. */
export type Profile = {
  account_id: number;
  login: string;
  /** The profile the machine acts as: at most one, and one whenever any exists. */
  active: boolean;
  /** Added beside the machine's own on purpose, so what it syncs is a choice. */
  chosen: boolean;
};

/** How far one row of what signing in sets up has got. */
export type SetUpState = "waiting" | "working" | "done";

/** What signing in sets up, row by row. `home` waits until the shared home exists. */
export type SetUpSteps = { identity: SetUpState; store: SetUpState; home: SetUpState };

/** How a sign-in that is no longer in flight ended. */
export type SignInOutcome = "signed_in" | "denied" | "expired" | "unreachable" | "refused";

/** The last sign-in's end, held until the next one starts. `said` is absent for one that succeeded. */
export type SignInEnded = { outcome: SignInOutcome; said?: string };

/** The sign-in in flight, tagged on `phase`. */
export type AccountFlow =
  | { phase: "idle"; ended?: SignInEnded }
  | { phase: "code"; code: string; address: string; expires_in_seconds: number }
  | { phase: "setup"; login: string; repository: string; steps: SetUpSteps };

/** `GET /accounts`, and what every act on the account answers with. Profiles are in the order they were joined. */
export type Accounts = { profiles: Profile[]; flow: AccountFlow };

/** `POST /accounts/sign_out`: which profile. */
export type SignOut = { account_id: number };

/** What `sign_out` answers. `store_row_remains` is true where the store could not be asked to forget the device. */
export type SignedOut = { accounts: Accounts; store_row_remains: boolean; said?: string };

/** `POST /accounts/set_active_account`: which profile the machine acts as. */
export type SetActiveAccount = { account_id: number };
