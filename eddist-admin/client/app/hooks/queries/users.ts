import {
  useMutation,
  useSuspenseQuery,
} from "@tanstack/react-query";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_USER_SEARCH = "/users/search/";

export const getUserSearch = ({
  params,
}: UseQueryOptions<paths[typeof GET_USER_SEARCH]["get"]>) => {
  return useSuspenseQuery({
    queryKey: [GET_USER_SEARCH],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_USER_SEARCH, {
        params,
        signal,
      });
      return data;
    },
  });
};

const UPDATE_USER_STATUS = "/users/{user_id}/status/";

export const useUpdateUserStatus = () => {
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_USER_STATUS]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_USER_STATUS, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
  });
};
