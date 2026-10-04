import type { useCaptchaPosting } from "../hooks/useCaptchaPosting";
import CaptchaWidget from "./CaptchaWidget";

interface CaptchaSectionProps {
  captcha: ReturnType<typeof useCaptchaPosting>;
}

/** Renders the captcha widgets (and retry error) for a posting flow, shared by PostThreadModal and PostResponseModal. */
const CaptchaSection = ({ captcha }: CaptchaSectionProps) => {
  if (captcha.configs.length === 0) return null;

  return (
    <div className="space-y-2">
      {captcha.error && (
        <p className="text-sm text-red-600 dark:text-red-400">
          認証に失敗しました。もう一度認証を行ってください。
        </p>
      )}
      {captcha.configs.map((config) => (
        <CaptchaWidget
          key={`${config.widget.form_field_name}-${captcha.widgetKey}`}
          config={config}
          onToken={captcha.onToken}
        />
      ))}
    </div>
  );
};

export default CaptchaSection;
