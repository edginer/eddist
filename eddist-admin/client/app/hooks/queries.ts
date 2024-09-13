import { useSuspenseQuery } from "@tanstack/react-query";
import type { ParamsOption, RequestBodyOption } from "openapi-fetch";
import type { paths } from "../openapi/schema";
import client from "../openapi/client";

type UseQueryOptions<T> = ParamsOption<T> &
  RequestBodyOption<T> & {
    reactQuery?: {
      enabled: boolean;
    };
  };

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
        signal, // allows React Query to cancel request
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

const CREATE_BOARD = "/boards/";

export const createBoard = ({
  body,
}: UseQueryOptions<paths[typeof CREATE_BOARD]["post"]>) => {
  const mutate = async () => {
    await client.POST(CREATE_BOARD, {
      body,
    });
  };
  return { mutate };
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

const DELETE_AUTHED_TOKEN = "/authed_tokens/{authed_token_id}/";

export const deleteAuthedToken = ({
  params,
}: UseQueryOptions<paths[typeof DELETE_AUTHED_TOKEN]["delete"]>) => {
  // simple fetch mutation without React Query
  const mutate = async () => {
    await client.DELETE(DELETE_AUTHED_TOKEN, {
      params,
    });
  };
  return { mutate };
};

const UPDATE_RESPONSE =
  "/boards/{board_key}/threads/{thread_id}/responses/{res_id}/";

export const updateResponse = ({
  params,
  body,
}: UseQueryOptions<paths[typeof UPDATE_RESPONSE]["patch"]>) => {
  const mutate = async () => {
    await client.PATCH(UPDATE_RESPONSE, {
      params,
      body,
    });
  };
  return { mutate };
};

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
      return data;
    },
  });
};

const CREATE_NG_WORDS = "/ng_words/";

export const createNgWord = ({
  body,
}: UseQueryOptions<paths[typeof CREATE_NG_WORDS]["post"]>) => {
  const mutate = async () => {
    await client.POST(CREATE_NG_WORDS, {
      body,
    });
  };
  return { mutate };
};

const DELETE_NG_WORD = "/ng_words/{ng_word_id}/";

export const deleteNgWord = ({
  params,
}: UseQueryOptions<paths[typeof DELETE_NG_WORD]["delete"]>) => {
  const mutate = async () => {
    await client.DELETE(DELETE_NG_WORD, {
      params,
    });
  };
  return { mutate };
};

const UPDATE_NG_WORD = "/ng_words/{ng_word_id}/";

export const updateNgWord = ({
  params,
  body,
}: UseQueryOptions<paths[typeof UPDATE_NG_WORD]["patch"]>) => {
  const mutate = async () => {
    await client.PATCH(UPDATE_NG_WORD, {
      params,
      body,
    });
  };
  return { mutate };
};
