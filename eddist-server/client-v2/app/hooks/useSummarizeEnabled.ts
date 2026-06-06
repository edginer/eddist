import { useState } from "react";

const STORAGE_KEY = "eddist:summarize:enabled";

export const useSummarizeEnabled = () => {
  const [enabled, setEnabledState] = useState(() => {
    if (typeof localStorage === "undefined") return false;
    return localStorage.getItem(STORAGE_KEY) === "true";
  });

  const setEnabled = (next: boolean) => {
    localStorage.setItem(STORAGE_KEY, String(next));
    setEnabledState(next);
  };

  return { enabled, setEnabled };
};
