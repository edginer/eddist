import { Button } from "flowbite-react";
import { useState } from "react";
import { FaArrowLeft } from "react-icons/fa";
import { Link, useNavigate, useParams } from "react-router";
import { twMerge } from "tailwind-merge";
import useSWR from "swr";
import PostThreadModal from "../components/PostThreadModal";
import type { Route } from "./+types/ThreadListPage";
import { fetchBoards, type Board } from "~/api-client/board";
import { fetchThreadList, type Thread } from "~/api-client/thread_list";

export const headers = (_: Route.HeadersArgs) => {
  return {
    "X-Frame-Options": "DENY",
    "X-Content-Type-Options": "nosniff",
    "Cache-Control": "max-age=5, s-maxage=1",
  };
};

const convertLinuxTimeToDateString = (linuxTime: number): string => {
  const date = new Date(linuxTime * 1000);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hour = String(date.getHours()).padStart(2, "0");
  const minute = String(date.getMinutes()).padStart(2, "0");
  return `${year}/${month}/${day} ${hour}:${minute}`;
};

export const loader = async ({ params, context }: Route.LoaderArgs) => {
  const [threadList, boards] = await Promise.all([
    fetchThreadList(params.boardKey!, {
      baseUrl:
        context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL,
    }),
    fetchBoards({
      baseUrl:
        context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL,
    }),
  ]);

  return {
    threadList,
    boards,
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
      availableUserRegistration: context.ENABLE_USER_REGISTRATION ?? false,
    },
  } satisfies {
    threadList: Thread[];
    boards: Board[];
    eddistData: {
      bbsName: string;
      availableUserRegistration: boolean;
    };
  };
};

const Meta = ({
  bbsName,
  boardName,
}: {
  bbsName: string;
  boardName: string;
}) => (
  <>
    <title>{`${boardName} - ${bbsName}`}</title>
    <meta property="og:title" content={`${bbsName} | ${boardName}`} />
    <meta property="og:site_name" content={bbsName} />
    <meta property="og:type" content="website" />
    <meta name="twitter:title" content={`${bbsName} | ${boardName}`} />
  </>
);

const ThreadListPage = ({
  loaderData: { threadList: data, boards, eddistData },
}: Route.ComponentProps) => {
  const params = useParams();
  const navigate = useNavigate();

  const { data: threadList, mutate } = useSWR(
    `${params.boardKey}/subject.txt`,
    () => fetchThreadList(params.boardKey!),
    {
      fallbackData: data,
    }
  );

  const [creatingThread, setCreatingThread] = useState(false);

  return (
    <div className="relative pt-16">
      <header
        className={
          "fixed top-0 left-0 right-0 z-50 bg-white shadow-md transition-transform duration-300 transform flex justify-between items-center p-3 lg:p-4"
        }
      >
        <Meta
          bbsName={eddistData.bbsName}
          boardName={
            boards?.find(
              (board: { board_key: string }) =>
                board.board_key === params.boardKey
            )?.name ?? "スレッド一覧"
          }
        />
        <Link to="/">
          <FaArrowLeft className="mx-2 mr-4 w-6 h-6" />
        </Link>
        <h1 className="text-2xl lg:text-4xl flex-grow truncate">
          {
            boards?.find(
              (board: { board_key: string }) =>
                board.board_key === params.boardKey
            )?.name
          }
        </h1>
        <Button
          onClick={() => setCreatingThread(true)}
          className={twMerge(
            "px-4 py-2 lg:px-6 lg:py-3 mx-2",
            params.boardKey || "hidden"
          )}
        >
          スレッド作成
        </Button>
      </header>

      <PostThreadModal
        boardKey={params.boardKey!}
        open={creatingThread}
        setOpen={setCreatingThread}
        refetchThreadList={mutate}
      />

      <div className="flex flex-col lg:flex-grow">
        {threadList.map((thread, i) => (
          <div key={thread.id} className="block">
            {i !== 0 && (
              <div className="border-b border-gray-400 lg:border-none lg:pt-2"></div>
            )}
            <button
              type="button"
              key={thread.id}
              className="hover:bg-gray-200 cursor-default text-left block w-full bg-gray-100 p-2 lg:p-3"
              onClick={() => {
                navigate(`/${params.boardKey}/${thread.id}`);
              }}
            >
              <div>
                <span
                  className="break-all"
                  dangerouslySetInnerHTML={{
                    __html: thread.title,
                  }}
                />
                <span> ({thread.responseCount})</span>
              </div>
              <div>
                <span>{convertLinuxTimeToDateString(thread.id)}</span>
                {thread.authorId && <span> ID:{thread.authorId}</span>}
              </div>
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ThreadListPage;
