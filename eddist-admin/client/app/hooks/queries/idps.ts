import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_IDPS = "/idps/";

export const getIdps = ({
  reactQuery,
}: UseQueryOptions<paths[typeof GET_IDPS]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_IDPS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_IDPS, {
        signal,
      });
      return data;
    },
  });
};

const GET_IDP = "/idps/{id}/";

export const getIdp = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_IDP]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_IDP, params.path.id],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_IDP, {
        params,
        signal,
      });
      return data;
    },
  });
};

const CREATE_IDP = "/idps/";

export const useCreateIdp = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_IDP]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_IDP, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_IDPS] });
      toast.success("IdP created successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to create IdP";
      toast.error(message);
    },
  });
};

const UPDATE_IDP = "/idps/{id}/";

export const useUpdateIdp = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_IDP]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_IDP, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_IDPS] });
      toast.success("IdP updated successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to update IdP";
      toast.error(message);
    },
  });
};

const DELETE_IDP = "/idps/{id}/";

export const useDeleteIdp = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_IDP]["delete"]>,
    ) => {
      await client.DELETE(DELETE_IDP, {
        params: args.params,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_IDPS] });
      toast.success("IdP deleted successfully");
    },
    onError: () => toast.error("Failed to delete IdP"),
  });
};
