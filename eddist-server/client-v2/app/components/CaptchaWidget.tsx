import { useEffect, useRef } from "react";
import type { CaptchaConfigPublic } from "../api-client/captcha-config";

interface CaptchaWidgetProps {
  config: CaptchaConfigPublic;
  onToken: (fieldName: string, token: string) => void;
}

const resolvePlaceholders = (template: string, siteKey: string, baseUrl?: string): string =>
  template.replace(/\{\{site_key\}\}/g, siteKey).replace(/\{\{base_url\}\}/g, baseUrl ?? "");

const loadScript = (src: string): void => {
  if (document.head.querySelector(`script[src="${CSS.escape(src)}"]`)) return;
  const script = document.createElement("script");
  script.src = src;
  script.async = true;
  document.head.appendChild(script);
};

/**
 * Renders a third-party captcha widget (Turnstile/hCaptcha/Cap/etc.) from
 * server-provided metadata, and reports the token once the widget populates
 * its hidden `<input name={form_field_name}>`.
 */
const CaptchaWidget = ({ config, onToken }: CaptchaWidgetProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const onTokenRef = useRef(onToken);
  onTokenRef.current = onToken;

  const scriptUrl = resolvePlaceholders(config.widget.script_url, config.site_key, config.base_url);
  const widgetHtml = resolvePlaceholders(
    config.widget.widget_html,
    config.site_key,
    config.base_url,
  );
  const scriptHandler = config.widget.script_handler
    ? resolvePlaceholders(config.widget.script_handler, config.site_key, config.base_url)
    : undefined;

  useEffect(() => {
    if (scriptUrl) loadScript(scriptUrl);
  }, [scriptUrl]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let handlerScript: HTMLScriptElement | undefined;
    if (scriptHandler) {
      handlerScript = document.createElement("script");
      handlerScript.textContent = scriptHandler;
      container.appendChild(handlerScript);
    }

    const fieldName = config.widget.form_field_name;
    let reported = false;
    const reportIfPresent = () => {
      if (reported) return;
      const input = container.querySelector<HTMLInputElement>(
        `input[name="${CSS.escape(fieldName)}"]`,
      );
      if (input?.value) {
        reported = true;
        onTokenRef.current(fieldName, input.value);
      }
    };

    reportIfPresent();
    const observer = new MutationObserver(reportIfPresent);
    observer.observe(container, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["value"],
    });

    return () => {
      observer.disconnect();
      handlerScript?.remove();
    };
  }, [config.widget.form_field_name, scriptHandler]);

  return (
    <div
      ref={containerRef}
      className="captcha-widget"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: widget HTML comes from server-side captcha provider config, not user input
      dangerouslySetInnerHTML={{ __html: widgetHtml }}
    />
  );
};

export default CaptchaWidget;
