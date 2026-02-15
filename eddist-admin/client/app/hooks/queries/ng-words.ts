import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_NG_WORDS = "/ng_words/";

export const getNgWords = ({
  params,
}: UseQueryOptions<paths[typeof GET_NG_WORDS]["get"]>) => {
  return useSuspenseQuery({
    queryKey: [GET_NG_WORDS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_NG_WORDS, {
        params,
        signal,
      });

      if (data) {
        data.sort((a, b) => {
          if (a.id < b.id) {
            return -1;
          }
          if (a.id > b.id) {
            return 1;
          }
          return 0;
        });
      }

      return data;
    },
  });
};

const CREATE_NG_WORDS = "/ng_words/";

export const useCreateNgWord = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_NG_WORDS]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_NG_WORDS, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_NG_WORDS] });
      toast.success("Successfully created NG word");
    },
    onError: () => toast.error("Failed to create NG word"),
  });
};

const UPDATE_NG_WORD = "/ng_words/{ng_word_id}/";

export const useUpdateNgWord = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_NG_WORD]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_NG_WORD, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_NG_WORDS] });
      toast.success("Successfully updated NG word");
    },
    onError: () => toast.error("Failed to update NG word"),
  });
};

const DELETE_NG_WORD = "/ng_words/{ng_word_id}/";

export const useDeleteNgWord = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_NG_WORD]["delete"]>,
    ) => {
      await client.DELETE(DELETE_NG_WORD, {
        params: args.params,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_NG_WORDS] });
      toast.success("Successfully deleted NG word");
    },
    onError: () => toast.error("Failed to delete NG word"),
  });
};
