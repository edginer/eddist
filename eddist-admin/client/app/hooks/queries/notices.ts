import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_NOTICES = "/notices/";

export const getNotices = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_NOTICES]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_NOTICES],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_NOTICES, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_NOTICE = "/notices/{id}/";

export const getNotice = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_NOTICE]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_NOTICE, params.path.id],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_NOTICE, {
        params,
        signal,
      });
      return data;
    },
  });
};

const CREATE_NOTICE = "/notices/";

export const useCreateNotice = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_NOTICE]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_NOTICE, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_NOTICES] });
      toast.success("Notice created successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to create notice";
      toast.error(message);
    },
  });
};

const UPDATE_NOTICE = "/notices/{id}/";

export const useUpdateNotice = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_NOTICE]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_NOTICE, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_NOTICES] });
      toast.success("Notice updated successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to update notice";
      toast.error(message);
    },
  });
};

const DELETE_NOTICE = "/notices/{id}/";

export const useDeleteNotice = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_NOTICE]["delete"]>,
    ) => {
      await client.DELETE(DELETE_NOTICE, {
        params: args.params,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_NOTICES] });
      toast.success("Notice deleted successfully");
    },
    onError: () => toast.error("Failed to delete notice"),
  });
};
