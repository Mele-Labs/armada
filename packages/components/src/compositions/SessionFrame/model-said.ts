const named = (family: string, version: string): string => `${family.charAt(0).toUpperCase()}${family.slice(1)} ${version}`;

/**
 * The name a model is shown by. A full id the CLI resolved is read as it is written, whoever prefixes it: `vendor-opus-5-5` as
 * "Opus 5.5" and `vendor-3-5-sonnet-20241022` as "Sonnet 3.5"; an alias a person picked is capitalised, and
 * anything else is shown as it came. **Nothing is filled in** for a model that was not said. The composer's Model
 * select reads the resolved model with it.
 */
export function modelSaid(model: string): string {
  const after = /^[a-z]+-(opus|sonnet|haiku)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/.exec(model);
  if (after !== null) return named(after[1]!, after[3] === undefined ? after[2]! : `${after[2]}.${after[3]}`);
  const before = /^[a-z]+-(\d+)(?:-(\d{1,2}))?-(opus|sonnet|haiku)(?:-\d{8})?$/.exec(model);
  if (before !== null) return named(before[3]!, before[2] === undefined ? before[1]! : `${before[1]}.${before[2]}`);
  return /^(opus|sonnet|haiku)$/.test(model) ? model.charAt(0).toUpperCase() + model.slice(1) : model;
}
