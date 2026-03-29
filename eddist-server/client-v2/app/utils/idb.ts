const DB_NAME = "eddist-client";
const DB_VERSION = 1;

const isBrowser = typeof window !== "undefined";

let dbPromise: Promise<IDBDatabase> | null = null;

export const openDB = (): Promise<IDBDatabase> => {
  if (!isBrowser) return Promise.reject(new Error("IndexedDB is not available on the server"));
  if (dbPromise) return dbPromise;

  dbPromise = new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;

      if (!db.objectStoreNames.contains("read_history")) {
        const historyStore = db.createObjectStore("read_history", { keyPath: "key" });
        historyStore.createIndex("visitedAt", "visitedAt");
      }

      if (!db.objectStoreNames.contains("favorites")) {
        const favoritesStore = db.createObjectStore("favorites", { keyPath: "key" });
        favoritesStore.createIndex("favoritedAt", "favoritedAt");
      }

      if (!db.objectStoreNames.contains("post_history")) {
        const postHistoryStore = db.createObjectStore("post_history", { keyPath: "key" });
        postHistoryStore.createIndex("postedAt", "postedAt");
      }
    };

    request.onsuccess = (event) => {
      resolve((event.target as IDBOpenDBRequest).result);
    };

    request.onerror = (event) => {
      dbPromise = null;
      reject((event.target as IDBOpenDBRequest).error);
    };
  });

  return dbPromise;
};

export const idbGet = <T>(storeName: string, key: string): Promise<T | undefined> => {
  return openDB().then(
    (db) =>
      new Promise<T | undefined>((resolve, reject) => {
        const tx = db.transaction(storeName, "readonly");
        const request = tx.objectStore(storeName).get(key);
        request.onsuccess = () => resolve(request.result as T | undefined);
        request.onerror = () => reject(request.error);
      }),
  );
};

export const idbPut = <T>(storeName: string, value: T): Promise<void> => {
  return openDB().then(
    (db) =>
      new Promise<void>((resolve, reject) => {
        const tx = db.transaction(storeName, "readwrite");
        const request = tx.objectStore(storeName).put(value);
        request.onsuccess = () => resolve();
        request.onerror = () => reject(request.error);
      }),
  );
};

export const idbDelete = (storeName: string, key: string): Promise<void> => {
  return openDB().then(
    (db) =>
      new Promise<void>((resolve, reject) => {
        const tx = db.transaction(storeName, "readwrite");
        const request = tx.objectStore(storeName).delete(key);
        request.onsuccess = () => resolve();
        request.onerror = () => reject(request.error);
      }),
  );
};

export const idbGetAll = <T>(storeName: string): Promise<T[]> => {
  return openDB().then(
    (db) =>
      new Promise<T[]>((resolve, reject) => {
        const tx = db.transaction(storeName, "readonly");
        const request = tx.objectStore(storeName).getAll();
        request.onsuccess = () => resolve(request.result as T[]);
        request.onerror = () => reject(request.error);
      }),
  );
};

export const idbCount = (storeName: string): Promise<number> => {
  return openDB().then(
    (db) =>
      new Promise<number>((resolve, reject) => {
        const tx = db.transaction(storeName, "readonly");
        const request = tx.objectStore(storeName).count();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      }),
  );
};
