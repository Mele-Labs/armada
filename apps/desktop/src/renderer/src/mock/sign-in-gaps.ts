// The base the sign-in gap scenarios share: the home machine on first run, `onboarding-home`, with the
// account the scenario carries in place of its own. The scenarios `sign-in-ended`, `sign-out`,
// `sign-in-no-home` is built on it.

import type { MockDraft, Scenario } from "./moment";
import { onboardingHome } from "./scenarios/onboarding-home";

/** `onboarding-home` under another name, signing in as it does but for what `account` changes. */
export function signInGap(name: string, says: string, account: Partial<NonNullable<MockDraft["account"]>>): Scenario {
  const base = onboardingHome.draft?.account;
  if (base === undefined) throw new Error("onboarding-home carries no account");
  return {
    ...onboardingHome,
    name,
    says,
    draft: { ...(onboardingHome.draft ?? {}), account: { ...base, ...account } },
  };
}
