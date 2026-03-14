export interface ClientConfig {
  enable_user_registration: boolean;
}

export const fetchClientConfig = async (options?: {
  baseUrl: string;
}): Promise<ClientConfig> => {
  return await fetch(
    `${(import.meta.env.SSR && options?.baseUrl) || ""}/api/client-config`,
  ).then((res) => res.json() satisfies Promise<ClientConfig>);
};
