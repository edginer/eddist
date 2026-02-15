import type { ParamsOption, RequestBodyOption } from "openapi-fetch";

export type UseQueryOptions<T> = ParamsOption<T> &
  RequestBodyOption<T> & {
    reactQuery?: {
      enabled: boolean;
    };
  };
