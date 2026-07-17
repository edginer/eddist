// Shared NG ID sync. Client-only, best-effort: the same-origin fetch attaches the
// edge-token cookie automatically, and failures must not block the local NG experience.

export const addSharedNgId = async (boardKey: string, ngId: string): Promise<void> => {
  try {
    await fetch(`/api/${boardKey}/ng-ids`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ng_id: ngId }),
    });
  } catch (e) {
    console.error("[sharedNgId] add failed", e);
  }
};

export const deleteSharedNgId = async (boardKey: string, ngId: string): Promise<void> => {
  try {
    await fetch(`/api/${boardKey}/ng-ids/${encodeURIComponent(ngId)}`, {
      method: "DELETE",
    });
  } catch (e) {
    console.error("[sharedNgId] delete failed", e);
  }
};
