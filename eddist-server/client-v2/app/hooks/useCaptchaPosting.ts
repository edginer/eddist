import { useEffect, useState } from "react";
import {
  type CaptchaConfigPublic,
  type CaptchaUsage,
  fetchCaptchaConfigs,
} from "../api-client/captcha-config";

/**
 * Loads the captcha widgets required for a posting flow (thread/response
 * creation) and tracks the tokens collected from them, including the
 * retry-on-failure cycle shared by PostThreadModal and PostResponseModal.
 */
export const useCaptchaPosting = (usage: CaptchaUsage, open: boolean) => {
  const [configs, setConfigs] = useState<CaptchaConfigPublic[]>([]);
  const [tokens, setTokens] = useState<Record<string, string>>({});
  const [error, setError] = useState(false);
  const [widgetKey, setWidgetKey] = useState(0);

  useEffect(() => {
    if (!open) return;
    setTokens({});
    setError(false);
    fetchCaptchaConfigs(usage)
      .then(setConfigs)
      .catch(() => setConfigs([]));
  }, [open, usage]);

  const isPending = configs.some((c) => !tokens[c.widget.form_field_name]);

  const onToken = (fieldName: string, token: string) => {
    setError(false);
    setTokens((prev) => ({ ...prev, [fieldName]: token }));
  };

  const onFailure = () => {
    setError(true);
    setTokens({});
    setWidgetKey((k) => k + 1);
  };

  return { configs, tokens, error, widgetKey, isPending, onToken, onFailure };
};
