import createClient, { type Middleware } from "openapi-fetch";
import type { paths } from "./schema";

export class ApiClientError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: unknown,
  ) {
    super(typeof body === "string" ? body : JSON.stringify(body));
    this.name = "ApiClientError";
  }
}

const throwOnError: Middleware = {
  async onResponse({ response }) {
    if (response.status >= 400) {
      const body = response.headers.get("content-type")?.includes("json")
        ? await response.clone().json()
        : await response.clone().text();
      throw new ApiClientError(response.status, body);
    }
    return undefined;
  },
};

const client = createClient<paths>({
  baseUrl: "/api/",
});

client.use(throwOnError);

export default client;
