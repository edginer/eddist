import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";

const LIST_AUTHED_TOKENS = "/authed_tokens";

export const listAuthedTokens = (
  params: paths[typeof LIST_AUTHED_TOKENS]["get"]["parameters"]["query"],
) => {
  return useQuery({
    queryKey: [LIST_AUTHED_TOKENS, params],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(LIST_AUTHED_TOKENS, {
        params: { query: params },
        signal,
      });
      return data;
    },
    placeholderData: keepPreviousData,
  });
};

const DELETE_AUTHED_TOKEN = "/authed_tokens/{authed_token_id}/";

export const useDeleteAuthedToken = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (args: {
      authedTokenId: string;
      usingOriginIp: boolean;
    }) => {
      await client.DELETE(DELETE_AUTHED_TOKEN, {
        params: {
          path: { authed_token_id: args.authedTokenId },
          query: { using_origin_ip: args.usingOriginIp },
        },
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [LIST_AUTHED_TOKENS] });
      toast.success("Successfully deleted authed token");
    },
    onError: () => toast.error("Failed to delete authed token"),
  });
};
