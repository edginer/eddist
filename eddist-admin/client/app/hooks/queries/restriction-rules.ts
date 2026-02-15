import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_RESTRICTION_RULES = "/restriction_rules";

export const getRestrictionRules = ({
  params,
}: UseQueryOptions<paths[typeof GET_RESTRICTION_RULES]["get"]>) => {
  return useSuspenseQuery({
    queryKey: [GET_RESTRICTION_RULES],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_RESTRICTION_RULES, {
        params,
        signal,
      });

      if (data) {
        data.sort((a, b) => {
          if (a.created_at < b.created_at) {
            return 1;
          }
          if (a.created_at > b.created_at) {
            return -1;
          }
          return 0;
        });
      }

      return data;
    },
  });
};

const GET_RESTRICTION_RULE = "/restriction_rules/{rule_id}";

export const getRestrictionRule = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_RESTRICTION_RULE]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_RESTRICTION_RULE, params.path.rule_id],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_RESTRICTION_RULE, {
        params,
        signal,
      });
      return data;
    },
  });
};

const CREATE_RESTRICTION_RULE = "/restriction_rules";

export const useCreateRestrictionRule = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_RESTRICTION_RULE]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_RESTRICTION_RULE, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_RESTRICTION_RULES] });
      toast.success("Successfully created restriction rule");
    },
    onError: () => toast.error("Failed to create restriction rule"),
  });
};

const UPDATE_RESTRICTION_RULE = "/restriction_rules/{rule_id}";

export const useUpdateRestrictionRule = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_RESTRICTION_RULE]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_RESTRICTION_RULE, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_RESTRICTION_RULES] });
      toast.success("Successfully updated restriction rule");
    },
    onError: () => toast.error("Failed to update restriction rule"),
  });
};

const DELETE_RESTRICTION_RULE = "/restriction_rules/{rule_id}";

export const useDeleteRestrictionRule = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_RESTRICTION_RULE]["delete"]>,
    ) => {
      await client.DELETE(DELETE_RESTRICTION_RULE, {
        params: args.params,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_RESTRICTION_RULES] });
      toast.success("Successfully deleted restriction rule");
    },
    onError: () => toast.error("Failed to delete restriction rule"),
  });
};
