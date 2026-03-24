import { useMutation, useQueryClient, useSuspenseQuery } from "@tanstack/react-query";
import { toast } from "react-toastify";
import client from "~/openapi/client";
import type { paths } from "~/openapi/schema";
import type { UseQueryOptions } from "./types";

const GET_TERMS = "/terms/";

export const getTerms = () => {
  return useSuspenseQuery({
    queryKey: [GET_TERMS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_TERMS, {
        signal,
      });
      return data;
    },
  });
};

const UPDATE_TERMS = "/terms/";

export const useUpdateTerms = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (args: UseQueryOptions<paths[typeof UPDATE_TERMS]["put"]>) => {
      const { data } = await client.PUT(UPDATE_TERMS, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_TERMS] });
      toast.success("Terms updated successfully");
    },
    onError: (error: Error) => {
      const message = error?.message || "Failed to update terms";
      toast.error(message);
    },
  });
};
