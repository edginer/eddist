import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_BOARDS = "/boards/";

export const getBoards = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_BOARDS]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_BOARDS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_BOARDS, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_BOARD = "/boards/{board_key}/";

export const getBoard = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_BOARD]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_BOARD, params.path.board_key],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_BOARD, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_BOARD_INFO = "/boards/{board_key}/info/";

export const getBoardInfo = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_BOARD_INFO]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_BOARD_INFO, params.path.board_key],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_BOARD_INFO, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_THREADS = "/boards/{board_key}/threads/";

export const getThreads = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_THREADS]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_THREADS, params.path.board_key],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_THREADS, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_ARCHIVED_THREADS = "/boards/{board_key}/archives/";

export const getArchivedThreads = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_ARCHIVED_THREADS]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_ARCHIVED_THREADS, params.path.board_key],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_ARCHIVED_THREADS, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_THREAD = "/boards/{board_key}/threads/{thread_id}/";

export const getThread = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_THREAD]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_THREAD, params.path.board_key, params.path.thread_id],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_THREAD, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_ARCHIVED_THREAD = "/boards/{board_key}/archives/{thread_id}/";

export const getArchivedThread = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_ARCHIVED_THREAD]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [
      GET_ARCHIVED_THREAD,
      params.path.board_key,
      params.path.thread_id,
    ],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_ARCHIVED_THREAD, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_RESPONSES = "/boards/{board_key}/threads/{thread_id}/responses/";

export const getResponses = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_RESPONSES]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_RESPONSES, params.path.board_key, params.path.thread_id],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_RESPONSES, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_ARCHIVED_RESPONSES =
  "/boards/{board_key}/archives/{thread_id}/responses/";

export const getArchivedResponses = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_ARCHIVED_RESPONSES]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [
      GET_ARCHIVED_RESPONSES,
      params.path.board_key,
      params.path.thread_id,
    ],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_ARCHIVED_RESPONSES, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_DAT_ARCHIVED_THREAD =
  "/boards/{board_key}/dat-archives/{thread_number}/";

export const getDatArcvhiedThread = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_DAT_ARCHIVED_THREAD]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [
      GET_DAT_ARCHIVED_THREAD,
      params.path.board_key,
      params.path.thread_number,
    ],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_DAT_ARCHIVED_THREAD, {
        params,
        signal,
      });
      return data;
    },
  });
};

const GET_DAT_ADMIN_ARCHIVED_THREAD =
  "/boards/{board_key}/admin-dat-archives/{thread_number}/";

export const getDatAdminArchivedThread = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_DAT_ADMIN_ARCHIVED_THREAD]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [
      GET_DAT_ADMIN_ARCHIVED_THREAD,
      params.path.board_key,
      params.path.thread_number,
    ],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_DAT_ADMIN_ARCHIVED_THREAD, {
        params,
        signal,
      });
      return data;
    },
  });
};

// Mutations

const UPDATE_BOARD = "/boards/{board_key}/";

export const useUpdateBoard = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_BOARD]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_BOARD, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_BOARDS] });
      queryClient.invalidateQueries({ queryKey: [GET_BOARD] });
      queryClient.invalidateQueries({ queryKey: [GET_BOARD_INFO] });
      toast.success("Successfully updated");
    },
    onError: () => toast.error("Failed to update"),
  });
};

const CREATE_BOARD = "/boards/";

export const useCreateBoard = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_BOARD]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_BOARD, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_BOARDS] });
      toast.success("Successfully created board");
    },
    onError: () => toast.error("Failed to create board"),
  });
};

const UPDATE_RESPONSE =
  "/boards/{board_key}/threads/{thread_id}/responses/{res_id}/";

export const useUpdateResponse = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_RESPONSE]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_RESPONSE, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_RESPONSES] });
      toast.success("Successfully updated response");
    },
    onError: () => toast.error("Failed to update response"),
  });
};

const UPDATE_DAT_ARCHIVED_RESPONSE =
  "/boards/{board_key}/dat-archives/{thread_number}/responses/";

export const useUpdateDatArchivedResponse = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<
        paths[typeof UPDATE_DAT_ARCHIVED_RESPONSE]["patch"]
      >,
    ) => {
      const { data } = await client.PATCH(UPDATE_DAT_ARCHIVED_RESPONSE, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [GET_DAT_ADMIN_ARCHIVED_THREAD],
      });
      queryClient.invalidateQueries({
        queryKey: [GET_DAT_ARCHIVED_THREAD],
      });
      toast.success("Successfully updated response");
    },
    onError: () => toast.error("Failed to update response"),
  });
};

const DELETE_DAT_ARCHIVED_RESPONSE =
  "/boards/{board_key}/dat-archives/{thread_number}/responses/{res_order}/";

export const useDeleteDatArchivedResponse = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<
        paths[typeof DELETE_DAT_ARCHIVED_RESPONSE]["delete"]
      >,
    ) => {
      const { data } = await client.DELETE(DELETE_DAT_ARCHIVED_RESPONSE, {
        params: args.params,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [GET_DAT_ADMIN_ARCHIVED_THREAD],
      });
      queryClient.invalidateQueries({
        queryKey: [GET_DAT_ARCHIVED_THREAD],
      });
      toast.success("Successfully deleted response");
    },
    onError: () => toast.error("Failed to delete response"),
  });
};

const DELETE_DAT_ARCHIVED_THREAD =
  "/boards/{board_key}/dat-archives/{thread_number}/";

export const useDeleteDatArchivedThread = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_DAT_ARCHIVED_THREAD]["delete"]>,
    ) => {
      const { data } = await client.DELETE(DELETE_DAT_ARCHIVED_THREAD, {
        params: args.params,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [GET_DAT_ADMIN_ARCHIVED_THREAD],
      });
      queryClient.invalidateQueries({
        queryKey: [GET_DAT_ARCHIVED_THREAD],
      });
      toast.success("Successfully deleted thread");
    },
    onError: () => toast.error("Failed to delete thread"),
  });
};

const COMPACT_THREAD = "/boards/{board_key}/threads-compaction/";

export const useCompactThread = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof COMPACT_THREAD]["post"]>,
    ) => {
      const { data } = await client.POST(COMPACT_THREAD, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_THREADS] });
      toast.success("Successfully compacted thread");
    },
    onError: () => toast.error("Failed to compact thread"),
  });
};
