import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const SERVER_SETTINGS = "/server-settings/";

export const getServerSettings = () => {
  return useSuspenseQuery({
    queryKey: [SERVER_SETTINGS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(SERVER_SETTINGS, {
        signal,
      });
      return data;
    },
  });
};

export const useUpsertServerSetting = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof SERVER_SETTINGS]["put"]>,
    ) => {
      const { data } = await client.PUT(SERVER_SETTINGS, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [SERVER_SETTINGS] });
      toast.success("Server setting saved successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to save server setting";
      toast.error(message);
    },
  });
};
