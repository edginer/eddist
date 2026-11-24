export interface ClientConfig {
  enable_user_registration: boolean;
  enable_safe_mode: boolean;
}

interface AppMetadata {
  schema_version: number;
  domain: string;
  bbs_name: string;
  enabled_features: string[];
  version: string;
}

const APP_METADATA_PATH = "/.well-known/appspecific/cc.eddibb.eddist.json";

let _clientConfigCache: { data: ClientConfig; expiresAt: number } | null = null;

export const fetchClientConfig = async (options?: { baseUrl: string }): Promise<ClientConfig> => {
  const isSsr = import.meta.env.SSR;
  const now = Date.now();
  if (isSsr && _clientConfigCache && _clientConfigCache.expiresAt > now) {
    return _clientConfigCache.data;
  }

  const baseUrl = isSsr ? (options?.baseUrl ?? "") : "";
  const response = await fetch(`${baseUrl}${APP_METADATA_PATH}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch application metadata (${response.status})`);
  }

  const metadata: AppMetadata | null = await response.json();
  if (
    metadata?.schema_version !== 1 ||
    typeof metadata.domain !== "string" ||
    !metadata.domain ||
    typeof metadata.bbs_name !== "string" ||
    !Array.isArray(metadata.enabled_features) ||
    typeof metadata.version !== "string" ||
    !metadata.version
  ) {
    throw new Error("Invalid or unsupported application metadata");
  }
  if (!isSsr && metadata.domain !== window.location.hostname) {
    throw new Error("Application metadata does not match the current hostname");
  }

  const data: ClientConfig = {
    enable_user_registration: metadata.enabled_features.includes("user_registration"),
    enable_safe_mode: metadata.enabled_features.includes("safe_mode"),
  };
  if (isSsr) {
    _clientConfigCache = { data, expiresAt: now + 60_000 };
  }
  return data;
};
