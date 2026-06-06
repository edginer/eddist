import { useEffect, useState } from "react";

export const useSummarizerSupported = (): boolean => {
  const [supported, setSupported] = useState(false);

  useEffect(() => {
    setSupported(typeof Summarizer !== "undefined");
  }, []);

  return supported;
};
