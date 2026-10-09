// What this phone keeps between visits: the signing key and the id the Gateway gave it.
// IndexedDB, because a CryptoKey made with `extractable: false` can be stored there
// and nowhere else: the private half never exists as bytes the page can read.

const DB = "armada-pocket";
const STORE = "device";

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB, 1);
    request.onupgradeneeded = () => request.result.createObjectStore(STORE);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function run<T>(mode: IDBTransactionMode, use: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await open();
  try {
    return await new Promise<T>((resolve, reject) => {
      const request = use(db.transaction(STORE, mode).objectStore(STORE));
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  } finally {
    db.close();
  }
}

export const generateKey = (): Promise<CryptoKeyPair> =>
  crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, ["sign", "verify"]);

export const keepKey = (key: CryptoKey) => run("readwrite", (store) => store.put(key, "key"));
export const keepDeviceId = (id: string) => run("readwrite", (store) => store.put(id, "device_id"));

export async function paired(): Promise<{ key: CryptoKey; deviceId: string } | undefined> {
  const [key, deviceId] = await Promise.all([
    run<CryptoKey | undefined>("readonly", (store) => store.get("key")),
    run<string | undefined>("readonly", (store) => store.get("device_id")),
  ]);
  return key === undefined || deviceId === undefined ? undefined : { key, deviceId };
}

export const forget = () => run("readwrite", (store) => store.clear());
