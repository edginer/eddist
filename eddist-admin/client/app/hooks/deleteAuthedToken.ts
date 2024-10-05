import { useCallback } from "react";
import { deleteAuthedToken } from "./queries";
import { toast } from "react-toastify";

export const useDeleteAuthedToken = () =>
  useCallback(async (token: string, deleteAllSameOriginIp: boolean) => {
    try {
      const { mutate } = deleteAuthedToken({
        params: {
          path: {
            authed_token_id: token,
          },
          query: {
            using_origin_ip: deleteAllSameOriginIp,
          },
        },
      });
      await mutate();
      toast.success(`Successfully deleted authed token`);
    } catch (error) {
      toast.error(`Failed to delete authed token`);
      return error;
    }
  }, []);
