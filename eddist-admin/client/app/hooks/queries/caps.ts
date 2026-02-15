import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

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

export const useCreateCap = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_CAP]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_CAP, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_CAPS] });
      toast.success("Successfully created cap");
    },
    onError: () => toast.error("Failed to create cap"),
  });
};

const UPDATE_CAP = "/caps/{cap_id}/";

export const useUpdateCap = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_CAP]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_CAP, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_CAPS] });
      toast.success("Successfully updated Cap");
    },
    onError: () => toast.error("Failed to update Cap"),
  });
};

const DELETE_CAP = "/caps/{cap_id}/";

export const useDeleteCap = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_CAP]["delete"]>,
    ) => {
      await client.DELETE(DELETE_CAP, {
        params: args.params,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_CAPS] });
      toast.success("Successfully deleted Cap");
    },
    onError: () => toast.error("Failed to delete Cap"),
  });
};
