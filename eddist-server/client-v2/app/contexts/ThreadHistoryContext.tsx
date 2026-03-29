import { createContext, type ReactNode, useCallback, useContext, useEffect, useState } from "react";
import { idbCount, idbDelete, idbGetAll, idbPut } from "~/utils/idb";

const HISTORY_STORE = "read_history";
const FAVORITES_STORE = "favorites";
const POST_HISTORY_STORE = "post_history";
const HISTORY_MAX = 500;

const isBrowser = typeof window !== "undefined";

export interface ReadHistoryEntry {
  key: string; // `${boardKey}/${threadKey}`
  boardKey: string;
  threadKey: string;
  title: string;
  lastReadCount: number;
  visitedAt: number; // Date.now() ms
}

export interface FavoriteEntry {
  key: string; // `${boardKey}/${threadKey}`
  boardKey: string;
  threadKey: string;
  title: string;
  favoritedAt: number;
  lastReadCount: number;
}

export interface PostHistoryEntry {
  key: string; // `${boardKey}/${threadKey}/${postedAt}`
  boardKey: string;
  threadKey: string;
  threadTitle: string;
  name: string;
  mail: string;
  body: string;
  postedAt: number;
}

interface ThreadHistoryContextValue {
  history: ReadHistoryEntry[];
  favorites: FavoriteEntry[];
  postHistory: PostHistoryEntry[];
  recordVisit: (boardKey: string, threadKey: string, title: string, count: number) => void;
  removeHistoryEntry: (key: string) => void;
  clearHistory: () => void;
  getHistoryEntry: (boardKey: string, threadKey: string) => ReadHistoryEntry | undefined;
  toggleFavorite: (boardKey: string, threadKey: string, title: string, count: number) => void;
  isFavorite: (boardKey: string, threadKey: string) => boolean;
  removeFavorite: (key: string) => void;
  recordPost: (
    boardKey: string,
    threadKey: string,
    threadTitle: string,
    name: string,
    mail: string,
    body: string,
  ) => void;
  removePostHistoryEntry: (key: string) => void;
  clearPostHistory: () => void;
}

const ThreadHistoryContext = createContext<ThreadHistoryContextValue | null>(null);

export const ThreadHistoryProvider = ({ children }: { children: ReactNode }) => {
  const [history, setHistory] = useState<ReadHistoryEntry[]>([]);
  const [favorites, setFavorites] = useState<FavoriteEntry[]>([]);
  const [postHistory, setPostHistory] = useState<PostHistoryEntry[]>([]);

  // Load from IndexedDB on mount (client-side only)
  useEffect(() => {
    if (!isBrowser) return;

    idbGetAll<ReadHistoryEntry>(HISTORY_STORE)
      .then((entries) => {
        setHistory(entries.sort((a, b) => b.visitedAt - a.visitedAt));
      })
      .catch((err) => console.error("[ThreadHistoryContext] Failed to load history:", err));

    idbGetAll<FavoriteEntry>(FAVORITES_STORE)
      .then((entries) => {
        setFavorites(entries.sort((a, b) => b.favoritedAt - a.favoritedAt));
      })
      .catch((err) => console.error("[ThreadHistoryContext] Failed to load favorites:", err));

    idbGetAll<PostHistoryEntry>(POST_HISTORY_STORE)
      .then((entries) => {
        setPostHistory(entries.sort((a, b) => b.postedAt - a.postedAt));
      })
      .catch((err) => console.error("[ThreadHistoryContext] Failed to load post history:", err));
  }, []);

  const recordVisit = useCallback(
    (boardKey: string, threadKey: string, title: string, count: number) => {
      if (!isBrowser) return;

      const key = `${boardKey}/${threadKey}`;
      const entry: ReadHistoryEntry = {
        key,
        boardKey,
        threadKey,
        title,
        lastReadCount: count,
        visitedAt: Date.now(),
      };

      idbPut<ReadHistoryEntry>(HISTORY_STORE, entry).catch((err) =>
        console.error("[ThreadHistoryContext] Failed to save history entry:", err),
      );

      // Prune oldest entries if over limit
      idbCount(HISTORY_STORE).then((total) => {
        if (total > HISTORY_MAX) {
          idbGetAll<ReadHistoryEntry>(HISTORY_STORE).then((all) => {
            const sorted = all.sort((a, b) => a.visitedAt - b.visitedAt);
            const toDelete = sorted.slice(0, total - HISTORY_MAX);
            for (const old of toDelete) {
              idbDelete(HISTORY_STORE, old.key).catch(() => {});
            }
          });
        }
      });

      setHistory((prev) => {
        const filtered = prev.filter((e) => e.key !== key);
        return [entry, ...filtered];
      });
    },
    [],
  );

  const removeHistoryEntry = useCallback((key: string) => {
    if (!isBrowser) return;

    idbDelete(HISTORY_STORE, key).catch((err) =>
      console.error("[ThreadHistoryContext] Failed to delete history entry:", err),
    );
    setHistory((prev) => prev.filter((e) => e.key !== key));
  }, []);

  const clearHistory = useCallback(() => {
    if (!isBrowser) return;

    idbGetAll<ReadHistoryEntry>(HISTORY_STORE).then((all) => {
      for (const entry of all) {
        idbDelete(HISTORY_STORE, entry.key).catch(() => {});
      }
    });
    setHistory([]);
  }, []);

  const getHistoryEntry = useCallback(
    (boardKey: string, threadKey: string): ReadHistoryEntry | undefined => {
      const key = `${boardKey}/${threadKey}`;
      return history.find((e) => e.key === key);
    },
    [history],
  );

  const toggleFavorite = useCallback(
    (boardKey: string, threadKey: string, title: string, count: number) => {
      if (!isBrowser) return;

      const key = `${boardKey}/${threadKey}`;
      const existing = favorites.find((f) => f.key === key);

      if (existing) {
        idbDelete(FAVORITES_STORE, key).catch((err) =>
          console.error("[ThreadHistoryContext] Failed to delete favorite:", err),
        );
        setFavorites((prev) => prev.filter((f) => f.key !== key));
      } else {
        const historyEntry = history.find((e) => e.key === key);
        const entry: FavoriteEntry = {
          key,
          boardKey,
          threadKey,
          title,
          favoritedAt: Date.now(),
          lastReadCount: historyEntry?.lastReadCount ?? count,
        };

        idbPut<FavoriteEntry>(FAVORITES_STORE, entry).catch((err) =>
          console.error("[ThreadHistoryContext] Failed to save favorite:", err),
        );
        setFavorites((prev) => [entry, ...prev]);
      }
    },
    [favorites, history],
  );

  const isFavorite = useCallback(
    (boardKey: string, threadKey: string): boolean => {
      const key = `${boardKey}/${threadKey}`;
      return favorites.some((f) => f.key === key);
    },
    [favorites],
  );

  const removeFavorite = useCallback((key: string) => {
    if (!isBrowser) return;

    idbDelete(FAVORITES_STORE, key).catch((err) =>
      console.error("[ThreadHistoryContext] Failed to delete favorite:", err),
    );
    setFavorites((prev) => prev.filter((f) => f.key !== key));
  }, []);

  const recordPost = useCallback(
    (
      boardKey: string,
      threadKey: string,
      threadTitle: string,
      name: string,
      mail: string,
      body: string,
    ) => {
      if (!isBrowser) return;

      const postedAt = Date.now();
      const key = `${boardKey}/${threadKey}/${postedAt}`;
      const entry: PostHistoryEntry = {
        key,
        boardKey,
        threadKey,
        threadTitle,
        name,
        mail,
        body,
        postedAt,
      };

      idbPut<PostHistoryEntry>(POST_HISTORY_STORE, entry).catch((err) =>
        console.error("[ThreadHistoryContext] Failed to save post history entry:", err),
      );
      setPostHistory((prev) => [entry, ...prev]);
    },
    [],
  );

  const removePostHistoryEntry = useCallback((key: string) => {
    if (!isBrowser) return;

    idbDelete(POST_HISTORY_STORE, key).catch((err) =>
      console.error("[ThreadHistoryContext] Failed to delete post history entry:", err),
    );
    setPostHistory((prev) => prev.filter((e) => e.key !== key));
  }, []);

  const clearPostHistory = useCallback(() => {
    if (!isBrowser) return;

    idbGetAll<PostHistoryEntry>(POST_HISTORY_STORE).then((all) => {
      for (const entry of all) {
        idbDelete(POST_HISTORY_STORE, entry.key).catch(() => {});
      }
    });
    setPostHistory([]);
  }, []);

  // Sync favorite lastReadCount when history is updated
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentionally only watch history to avoid update loop
  useEffect(() => {
    if (favorites.length === 0 || history.length === 0) return;

    setFavorites((prevFavorites) =>
      prevFavorites.map((fav) => {
        const histEntry = history.find((h) => h.key === fav.key);
        if (histEntry && histEntry.lastReadCount !== fav.lastReadCount) {
          const updated = { ...fav, lastReadCount: histEntry.lastReadCount };
          idbPut<FavoriteEntry>(FAVORITES_STORE, updated).catch(() => {});
          return updated;
        }
        return fav;
      }),
    );
  }, [history]);

  const value: ThreadHistoryContextValue = {
    history,
    favorites,
    postHistory,
    recordVisit,
    removeHistoryEntry,
    clearHistory,
    getHistoryEntry,
    toggleFavorite,
    isFavorite,
    removeFavorite,
    recordPost,
    removePostHistoryEntry,
    clearPostHistory,
  };

  return <ThreadHistoryContext.Provider value={value}>{children}</ThreadHistoryContext.Provider>;
};

export const useThreadHistory = (): ThreadHistoryContextValue => {
  const context = useContext(ThreadHistoryContext);
  if (!context) {
    throw new Error("useThreadHistory must be used within ThreadHistoryProvider");
  }
  return context;
};
