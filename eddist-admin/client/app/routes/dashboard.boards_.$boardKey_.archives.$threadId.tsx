import { useCallback } from "react";
import ResponseList from "../components/ResponseList";
import { Link, useParams } from "@remix-run/react";
import Breadcrumb from "../components/Breadcrumb";
import { toast } from "react-toastify";
import {
  deleteAuthedToken,
  getArchivedResponses,
  getArchivedThread,
} from "~/hooks/queries";
import { useDeleteAuthedToken } from "~/hooks/deleteAuthedToken";

const Page = () => {
  const params = useParams();
  if (params.boardKey == null || params.threadId == null) {
    throw new Error("Page not found");
  }

  const { data: responses } = getArchivedResponses({
    params: {
      path: {
        board_key: params.boardKey,
        thread_id: Number(params.threadId),
      },
    },
  });
  const { data: thread } = getArchivedThread({
    params: {
      path: {
        board_key: params.boardKey,
        thread_id: Number(params.threadId),
      },
    },
  });

  const deleteAuthedCookie = useDeleteAuthedToken();

  return (
    <>
      <div className="p-4">
        <h1 className="text-3xl font-bold">
          Thread: {thread!.title} ({thread!.thread_number})
        </h1>
        <div className="flex justify-between">
          <Breadcrumb>
            <Link
              to="/dashboard/boards"
              className="text-gray-500 hover:text-gray-700"
            >
              Boards
            </Link>
            <Link
              to={`/dashboard/boards/${params.boardKey}`}
              className="text-gray-500 hover:text-gray-700"
            >
              {params.boardKey}
            </Link>
            <span className="text-gray-500" aria-current="page">
              {thread!.title}
            </span>
          </Breadcrumb>
          <Link className="my-2 mr-4 underline underline-offset-1" to={"./dat"}>
            Go to archive dat page
          </Link>
        </div>

        <ResponseList
          onClickDeleteAuthedToken={async (token) => {
            await deleteAuthedCookie(token, false);
          }}
          onClickDeleteAuthedTokensAssociatedWithIp={async (token) => {
            await deleteAuthedCookie(token, true);
          }}
          responses={responses!.filter((r) => r != null) ?? []}
        />
      </div>
    </>
  );
};

export default Page;
