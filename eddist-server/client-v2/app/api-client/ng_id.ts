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

export const addSharedThreadMetadent = async (
  boardKey: string,
  metadent: string,
): Promise<void> => {
  try {
    await fetch(`/api/${boardKey}/ng-thread-metadents`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ metadent }),
    });
  } catch (e) {
    console.error("[sharedThreadMetadent] add failed", e);
  }
};

// Mirrors MAX_NG_IDS_PER_DELETE in routes/ng_id.rs.
const DELETE_BATCH_SIZE = 200;

export const deleteSharedNgIds = async (boardKey: string, ngIds: string[]): Promise<void> => {
  for (let i = 0; i < ngIds.length; i += DELETE_BATCH_SIZE) {
    const batch = ngIds.slice(i, i + DELETE_BATCH_SIZE);

    try {
      await fetch(`/api/${boardKey}/ng-ids`, {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ng_ids: batch }),
      });
    } catch (e) {
      console.error("[sharedNgId] delete failed", e);
    }
  }
};

export const deleteSharedThreadMetadents = async (
  boardKey: string,
  metadents: string[],
): Promise<void> => {
  for (let i = 0; i < metadents.length; i += DELETE_BATCH_SIZE) {
    const batch = metadents.slice(i, i + DELETE_BATCH_SIZE);

    try {
      await fetch(`/api/${boardKey}/ng-thread-metadents`, {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ metadents: batch }),
      });
    } catch (e) {
      console.error("[sharedThreadMetadent] delete failed", e);
    }
  }
};
