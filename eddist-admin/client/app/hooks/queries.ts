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

const UPDATE_BOARD = "/boards/{board_key}/";

export const updateBoard = ({
  params,
  body,
}: UseQueryOptions<paths[typeof UPDATE_BOARD]["patch"]>) => {
  const mutate = async () => {
    await client.PATCH(UPDATE_BOARD, {
      params,
      body,
    });
  };
  return { mutate };
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

const UPDATE_DAT_ARCHIVED_RESPONSE =
  "/boards/{board_key}/dat-archives/{thread_number}/responses/";

export const updateDatArchivedResponse = ({
  params,
  body,
}: UseQueryOptions<paths[typeof UPDATE_DAT_ARCHIVED_RESPONSE]["patch"]>) => {
  const mutate = async () => {
    await client.PATCH(UPDATE_DAT_ARCHIVED_RESPONSE, {
      params,
      body,
    });
  };
  return { mutate };
};

const DELETE_DAT_ARCHIVED_RESPONSE =
  "/boards/{board_key}/dat-archives/{thread_number}/responses/{res_order}/";

export const deleteDatArchivedResponse = ({
  params,
}: UseQueryOptions<paths[typeof DELETE_DAT_ARCHIVED_RESPONSE]["delete"]>) => {
  const mutate = async () => {
    await client.DELETE(DELETE_DAT_ARCHIVED_RESPONSE, {
      params,
    });
  };
  return { mutate };
};

const DELETE_DAT_ARCHIVED_THREAD =
  "/boards/{board_key}/dat-archives/{thread_number}/";

export const deleteDatArchivedThread = ({
  params,
}: UseQueryOptions<paths[typeof DELETE_DAT_ARCHIVED_THREAD]["delete"]>) => {
  const mutate = async () => {
    await client.DELETE(DELETE_DAT_ARCHIVED_THREAD, {
      params,
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

const GET_CAPS = "/caps/";

export const getCaps = ({
  params,
}: UseQueryOptions<paths[typeof GET_CAPS]["get"]>) => {
  return useSuspenseQuery({
    queryKey: [GET_CAPS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_CAPS, {
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

const CREATE_CAP = "/caps/";

export const createCap = ({
  body,
}: UseQueryOptions<paths[typeof CREATE_CAP]["post"]>) => {
  const mutate = async () => {
    await client.POST(CREATE_CAP, {
      body,
    });
  };
  return { mutate };
};

const DELETE_CAP = "/caps/{cap_id}/";

export const deleteCap = ({
  params,
}: UseQueryOptions<paths[typeof DELETE_CAP]["delete"]>) => {
  const mutate = async () => {
    await client.DELETE(DELETE_CAP, {
      params,
    });
  };
  return { mutate };
};

const UPDATE_CAP = "/caps/{cap_id}/";

export const updateCap = ({
  params,
  body,
}: UseQueryOptions<paths[typeof UPDATE_CAP]["patch"]>) => {
  const mutate = async () => {
    await client.PATCH(UPDATE_CAP, {
      params,
      body,
    });
  };
  return { mutate };
};

const COMPACT_THREAD = "/boards/{board_key}/threads-compaction/";

export const compactThread = ({
  params,
  body,
}: UseQueryOptions<paths[typeof COMPACT_THREAD]["post"]>) => {
  const mutate = async () => {
    await client.POST(COMPACT_THREAD, {
      params,
      body,
    });
  };
  return { mutate };
};

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

export const updateUserStatus = ({
  params,
  body,
}: UseQueryOptions<paths[typeof UPDATE_USER_STATUS]["patch"]>) => {
  const mutate = async () => {
    await client.PATCH(UPDATE_USER_STATUS, {
      params,
      body,
    });
  };
  return { mutate };
};
