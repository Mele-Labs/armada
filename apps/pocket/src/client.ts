// Every call to the Gateway goes through `call`, which signs it. A 401 means the Gateway
// does not know this phone any more, so the app goes to /pair with the Gateway's sentence.

import { paired } from "./device";
import { go } from "./router";
import { signHeaders } from "./signing";

export class Refused extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}

const NOT_ANSWERING = "Armada is not answering on your Mac. Open it there, then try again.";

let why = "";
export const lastRefusal = () => why;

export async function call(path: string, init: { method?: string; body?: unknown; leavePairing?: boolean } = {}): Promise<Response> {
  const method = init.method ?? "GET";
  const body = init.body === undefined ? "" : JSON.stringify(init.body);
  const device = await paired();
  if (device === undefined) {
    if (init.leavePairing !== false) go("/pair");
    throw new Refused(401, "");
  }
  const headers: Record<string, string> = await signHeaders(device.key, device.deviceId, method, path, body);
  if (body !== "") headers["Content-Type"] = "application/json";
  let response: Response;
  try {
    response = await fetch(path, { method, headers, body: body === "" ? undefined : body });
  } catch {
    throw new Refused(0, NOT_ANSWERING);
  }
  if (response.status === 401) {
    why = await response.text();
    if (init.leavePairing !== false) go("/pair");
    throw new Refused(401, why);
  }
  if (!response.ok) throw new Refused(response.status, (await response.text()) || NOT_ANSWERING);
  return response;
}

export async function getJson<T>(path: string): Promise<T> {
  return (await call(path)).json() as Promise<T>;
}
