import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "react-toastify";
import client from "~/openapi/client";
import type { paths } from "~/openapi/schema";

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
    mutationFn: async (args: { authedTokenId: string; usingOriginIp: boolean }) => {
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

const REQUIRE_REAUTH = "/authed_tokens/{authed_token_id}/require-reauth";

export const useRequireReAuth = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (authedTokenId: string) => {
      await client.POST(REQUIRE_REAUTH, {
        params: { path: { authed_token_id: authedTokenId } },
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [LIST_AUTHED_TOKENS] });
      toast.success("Re-auth required for token");
    },
    onError: () => toast.error("Failed to set re-auth requirement"),
  });
};

export const useClearReAuth = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (authedTokenId: string) => {
      await client.DELETE(REQUIRE_REAUTH, {
        params: { path: { authed_token_id: authedTokenId } },
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [LIST_AUTHED_TOKENS] });
      toast.success("Re-auth requirement cleared");
    },
    onError: () => toast.error("Failed to clear re-auth requirement"),
  });
};
