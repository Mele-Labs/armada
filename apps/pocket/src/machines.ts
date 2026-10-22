// The Macs this phone is paired with, one per Mac, in the order they were paired: the first is the one
// whose push key the phone's one subscription was made with. IndexedDB, because a CryptoKey made with
// `extractable: false` can be stored there and nowhere else: the private half never exists as bytes.

export type Machine = {
  /** `https://<tailnet name>`, where every call to this Mac goes. */
  origin: string;
  /** What the Mac calls itself (`GET /api/machine`), shown beside its Jobs. */
  name: string;
  deviceId: string;
  key: CryptoKey;
};

const DB = "armada-pocket";
const OLD = "device";
const STORE = "machines";

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB, 2);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(OLD)) db.createObjectStore(OLD);
      if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE, { keyPath: "origin" });
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function run<T>(stores: string[], mode: IDBTransactionMode, use: (tx: IDBTransaction) => IDBRequest<T>): Promise<T> {
  const db = await open();
  try {
    return await new Promise<T>((resolve, reject) => {
      const request = use(db.transaction(stores, mode));
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  } finally {
    db.close();
  }
}

type Kept = Machine & { order: number };

/**
 * A phone paired before there were several Macs kept one key and one id, for the Mac that served the
 * app. It becomes that Mac's entry, named once the Mac says its name.
 */
async function carryOver(): Promise<Kept | undefined> {
  const key = await run<CryptoKey | undefined>([OLD], "readonly", (tx) => tx.objectStore(OLD).get("key"));
  const deviceId = await run<string | undefined>([OLD], "readonly", (tx) => tx.objectStore(OLD).get("device_id"));
  if (key === undefined || deviceId === undefined) return undefined;
  const kept: Kept = { origin: window.location.origin, name: "", deviceId, key, order: 0 };
  await run([STORE], "readwrite", (tx) => tx.objectStore(STORE).put(kept));
  await run([OLD], "readwrite", (tx) => tx.objectStore(OLD).clear());
  return kept;
}

export async function machines(): Promise<Machine[]> {
  const kept = await run<Kept[]>([STORE], "readonly", (tx) => tx.objectStore(STORE).getAll());
  if (kept.length === 0) {
    const old = await carryOver();
    if (old !== undefined) kept.push(old);
  }
  return kept.sort((a, b) => a.order - b.order).map(({ order: _order, ...machine }) => machine);
}

/** Keeps a Mac, or replaces the one at its origin and keeps its place. */
export async function keepMachine(machine: Machine): Promise<void> {
  const kept = await run<Kept[]>([STORE], "readonly", (tx) => tx.objectStore(STORE).getAll());
  const was = kept.find((one) => one.origin === machine.origin);
  const order = was?.order ?? kept.reduce((top, one) => Math.max(top, one.order + 1), 0);
  await run([STORE], "readwrite", (tx) => tx.objectStore(STORE).put({ ...machine, order }));
}

export const forgetMachine = (origin: string) => run([STORE], "readwrite", (tx) => tx.objectStore(STORE).delete(origin));
