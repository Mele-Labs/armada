// The account on the wire, mirrored by hand from `crates/ipc/src/accounts.rs`.

/** How a repository came to be bound to its profile: its owner matched, or a person chose. */
export type BoundHow = "matched" | "chosen";

/** A repository bound to a profile. `root` names it in `set_repository_profile`; `name` is the folder's own. */
export type BoundRepository = { root: string; name: string; how: BoundHow };

/** Where a profile's shared home is: the copy here where there is one (`made`), else where it will be made. `account` is the owner, the profile's login or one of its organisations. */
export type ProfileHome = { account: string; repository: string; made: boolean };

/** One profile this machine has joined. `account_id` is the forge's numeric id and names the profile in every act. There is no active profile: a repository is bound to one. */
export type Profile = {
  account_id: number;
  login: string;
  /** The organisations the account belonged to when it signed in, lowercase. Empty where the forge would not list them as well as where there are none. */
  orgs: string[];
  /** Added beside the machine's own on purpose, so what it syncs is a choice. */
  chosen: boolean;
  /** Where its shared home is. */
  home: ProfileHome;
  /** The repositories whose claims and syncing go to this profile's store view. */
  repositories: BoundRepository[];
};

/** A repository bound to no profile, so it claims and syncs nothing. `candidates` are the profiles its owner matched: none, or two where an organisation is shared. */
export type UnmatchedRepository = {
  root: string;
  name: string;
  candidates: number[];
  /** A person chose "None", so it is not waiting on a choice. */
  none_chosen: boolean;
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

/** `GET /accounts`, and what every act on the account answers with. Profiles are in the order they were joined; `unmatched` is every served repository bound to none. */
export type Accounts = { profiles: Profile[]; unmatched: UnmatchedRepository[]; flow: AccountFlow };

/** `POST /accounts/sign_out`: which profile. */
export type SignOut = { account_id: number };

/** What `sign_out` answers. `store_row_remains` is true where the store could not be asked to forget the device. */
export type SignedOut = { accounts: Accounts; store_row_remains: boolean; said?: string };

/** `POST /accounts/set_repository_profile`: which profile a repository is bound to. `account_id: null` is "None": the repository claims and syncs nothing. */
export type SetRepositoryProfile = { root: string; account_id: number | null };
