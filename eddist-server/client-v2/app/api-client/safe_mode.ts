export const fetchUnsafeThreadIds = async (
  boardKey: string,
  options?: { baseUrl?: string },
): Promise<Set<number>> => {
  const base = (import.meta.env.SSR && options?.baseUrl) || "";
  try {
    const res = await fetch(`${base}/api/${boardKey}/unsafe-thread-ids`);
    if (!res.ok) return new Set();
    const data = await res.json();
    return new Set<number>(data.thread_ids ?? []);
  } catch (e) {
    console.error("fetchUnsafeThreadIds failed:", e);
    return new Set();
  }
};
