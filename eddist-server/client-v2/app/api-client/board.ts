export interface Board {
  name: string;
  board_key: string;
  default_name: string;
}

let _boardsCache: { data: Board[]; expiresAt: number } | null = null;

export const fetchBoards = async (options?: {
  baseUrl: string;
}): Promise<Board[]> => {
  if (import.meta.env.SSR) {
    const now = Date.now();
    if (_boardsCache && _boardsCache.expiresAt > now) return _boardsCache.data;
    const data: Board[] = await fetch(
      `${options?.baseUrl ?? ""}/api/boards`
    ).then((r) => r.json());
    _boardsCache = { data, expiresAt: now + 60_000 };
    return data;
  }
  return fetch("/api/boards").then((r) => r.json());
};
