import { useState, useCallback } from "react";
import ResponseList from "../components/ResponseList";
import { Link, useParams } from "@remix-run/react";
import Breadcrumb from "../components/Breadcrumb";
import clsx from "clsx";
import { toast } from "react-toastify";
import {
  deleteAuthedToken,
  getArchivedResponses,
  getArchivedThread,
} from "~/hooks/queries";

const Page = () => {
  const params = useParams();
  if (params.boardKey == null || params.threadId == null) {
    throw new Error("Page not found");
  }

  const [showingFloatingDetail, setShowingFloatingDetail] = useState(false);

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

  const deleteAuthedCookie = useCallback(
    async (token: string, deleteAllSameOriginIp: boolean) => {
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
    },
    []
  );

  return (
    <>
      <div className="p-4">
        <h1 className="text-3xl font-bold">
          Thread: {thread!.title} ({thread!.thread_number})
        </h1>
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
        <ResponseList
          onClickDeleteAuthedToken={async (token) => {
            await deleteAuthedCookie(token, false);
          }}
          onClickDeleteAuthedTokensAssociatedWithIp={async (token) => {
            await deleteAuthedCookie(token, true);
          }}
          onClickEditResponse={(_) => {}}
          onClickAbon={(_) => {}}
          responses={responses!.filter((r) => r != null) ?? []}
          selectedResponses={[]}
          setSelectedResponses={(_) => {}}
        />
      </div>
      <div className="fixed bottom-8 right-8 z-10">
        <div className={clsx(showingFloatingDetail ? "block" : "hidden")}>
          <button>
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 6h18M6 6v-.01M9 6v-.01M12 6v-.01M15 6v-.01M18 6v-.01M6 6a2 2 0 012-2h8a2 2 0 012 2v.01M16 10v6a2 2 0 01-2 2H10a2 2 0 01-2-2v-6M14 3v-.01M10 3v-.01" />
            </svg>
          </button>
          <button>
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="feather feather-trash-2"
            >
              <path d="M3 6h18M16 10v6a2 2 0 01-2 2H10a2 2 0 01-2-2v-6M5 6h14l-1.5 12A2 2 0 0115.5 20h-7a2 2 0 01-1.96-1.56L5 6z"></path>
              <rect x="15" y="15" width="6" height="6" rx="1"></rect>
              <text
                x="19"
                y="20"
                fontSize="10"
                textAnchor="middle"
                fill="currentColor"
              >
                ALL
              </text>
              <rect
                x="17"
                y="18"
                width="16"
                height="10"
                rx="2"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              ></rect>
            </svg>
          </button>
        </div>
        <button
          className="rounded-full shadow-xl border-2 bg-blue-500 hover:bg-blue-700 w-14 h-14 items-center flex justify-center"
          onClick={() => setShowingFloatingDetail(!showingFloatingDetail)}
        >
          {showingFloatingDetail ? (
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6 text-white"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              width="24"
              height="24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          ) : (
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6 text-white"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              width="24"
              height="24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 6h16M4 12h16m-7 6h7"
              />
            </svg>
          )}
        </button>
      </div>
    </>
  );
};

export default Page;
