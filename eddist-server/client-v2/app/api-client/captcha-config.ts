export interface CaptchaWidgetMetadata {
  form_field_name: string;
  script_url: string;
  widget_html: string;
  script_handler?: string;
}

export interface CaptchaConfigPublic {
  provider: string;
  site_key: string;
  base_url?: string;
  widget: CaptchaWidgetMetadata;
}

export type CaptchaUsage = "thread_creation" | "res_creation";

const _captchaConfigCache = new Map<
  CaptchaUsage,
  { data: CaptchaConfigPublic[]; expiresAt: number }
>();

export const fetchCaptchaConfigs = async (usage: CaptchaUsage): Promise<CaptchaConfigPublic[]> => {
  const now = Date.now();
  const cached = _captchaConfigCache.get(usage);
  if (cached && cached.expiresAt > now) return cached.data;

  const res = await fetch(`/api/captcha-configs?usage=${usage}`);
  if (!res.ok) {
    throw new Error(`Failed to fetch captcha configs for usage "${usage}"`);
  }
  const data: CaptchaConfigPublic[] = await res.json();
  _captchaConfigCache.set(usage, { data, expiresAt: now + 60_000 });
  return data;
};
