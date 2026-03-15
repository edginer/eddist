export interface ClientConfig {
  enable_user_registration: boolean;
}

let _clientConfigCache: { data: ClientConfig; expiresAt: number } | null = null;

export const fetchClientConfig = async (options?: {
  baseUrl: string;
}): Promise<ClientConfig> => {
  if (import.meta.env.SSR) {
    const now = Date.now();
    if (_clientConfigCache && _clientConfigCache.expiresAt > now)
      return _clientConfigCache.data;
    const data: ClientConfig = await fetch(
      `${options?.baseUrl ?? ""}/api/client-config`
    ).then((r) => r.json());
    _clientConfigCache = { data, expiresAt: now + 60_000 };
    return data;
  }
  return fetch("/api/client-config").then((r) => r.json());
};
