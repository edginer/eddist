import {
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from "@tanstack/react-query";
import { toast } from "react-toastify";
import type { paths } from "~/openapi/schema";
import client from "~/openapi/client";
import type { UseQueryOptions } from "./types";

const GET_CAPTCHA_CONFIGS = "/captcha-configs/";

export const getCaptchaConfigs = () => {
  return useSuspenseQuery({
    queryKey: [GET_CAPTCHA_CONFIGS],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_CAPTCHA_CONFIGS, {
        signal,
      });
      return data;
    },
  });
};

const GET_CAPTCHA_CONFIG = "/captcha-configs/{id}/";

export const getCaptchaConfig = ({
  params,
  reactQuery,
}: UseQueryOptions<paths[typeof GET_CAPTCHA_CONFIG]["get"]>) => {
  return useSuspenseQuery({
    ...reactQuery,
    queryKey: [GET_CAPTCHA_CONFIG, params.path.id],
    queryFn: async ({ signal }) => {
      const { data } = await client.GET(GET_CAPTCHA_CONFIG, {
        params,
        signal,
      });
      return data;
    },
  });
};

const CREATE_CAPTCHA_CONFIG = "/captcha-configs/";

export const useCreateCaptchaConfig = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof CREATE_CAPTCHA_CONFIG]["post"]>,
    ) => {
      const { data } = await client.POST(CREATE_CAPTCHA_CONFIG, {
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_CAPTCHA_CONFIGS] });
      toast.success("Captcha config created successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to create captcha config";
      toast.error(message);
    },
  });
};

const UPDATE_CAPTCHA_CONFIG = "/captcha-configs/{id}/";

export const useUpdateCaptchaConfig = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof UPDATE_CAPTCHA_CONFIG]["patch"]>,
    ) => {
      const { data } = await client.PATCH(UPDATE_CAPTCHA_CONFIG, {
        params: args.params,
        body: args.body,
      });
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_CAPTCHA_CONFIGS] });
      toast.success("Captcha config updated successfully");
    },
    onError: (error: any) => {
      const message = error?.message || "Failed to update captcha config";
      toast.error(message);
    },
  });
};

const DELETE_CAPTCHA_CONFIG = "/captcha-configs/{id}/";

export const useDeleteCaptchaConfig = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (
      args: UseQueryOptions<paths[typeof DELETE_CAPTCHA_CONFIG]["delete"]>,
    ) => {
      await client.DELETE(DELETE_CAPTCHA_CONFIG, {
        params: args.params,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [GET_CAPTCHA_CONFIGS] });
      toast.success("Captcha config deleted successfully");
    },
    onError: () => toast.error("Failed to delete captcha config"),
  });
};
