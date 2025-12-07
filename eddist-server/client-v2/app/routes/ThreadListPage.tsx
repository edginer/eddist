import { Button } from "flowbite-react";
import { useMemo, useState } from "react";
import { FaArrowLeft, FaSort, FaSortDown, FaSortUp } from "react-icons/fa";
import { Link, useNavigate, useParams } from "react-router";
import { twMerge } from "tailwind-merge";
import useSWR from "swr";
import PostThreadModal from "../components/PostThreadModal";
import type { Route } from "./+types/ThreadListPage";
import { fetchBoards, type Board } from "~/api-client/board";
import { fetchThreadList, type Thread } from "~/api-client/thread_list";

type SortKey = "responseCount" | "speed" | "creationTime" | "lastUpdated";
type SortOrder = "asc" | "desc";

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

const calculateThreadSpeed = (
  threadId: number,
  responseCount: number,
  currentTime: number
): number => {
  const threadCreationTime = threadId * 1000;
  let ageInDays: number;
  if (currentTime - threadCreationTime < 500) {
    ageInDays = 1 / (60 * 60 * 24);
  } else {
    ageInDays = (currentTime - threadCreationTime) / (1000 * 60 * 60 * 24);
  }

  const speed = responseCount / ageInDays;
  return Math.round(speed * 100) / 100;
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
    currentTime: Date.now(),
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
      availableUserRegistration: context.ENABLE_USER_REGISTRATION ?? false,
    },
  } satisfies {
    threadList: Thread[];
    boards: Board[];
    currentTime: number;
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
  loaderData: { threadList: data, boards, currentTime, eddistData },
}: Route.ComponentProps) => {
  const params = useParams();
  const navigate = useNavigate();

  const { data: threadList, mutate } = useSWR(
    `${params.boardKey}/subject.txt`,
    () => fetchThreadList(params.boardKey!),
    {
      fallbackData: data,
      revalidateOnMount: false,
    }
  );

  const [creatingThread, setCreatingThread] = useState(false);
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortOrder, setSortOrder] = useState<SortOrder>("desc");
  const [showSortControls, setShowSortControls] = useState(false);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortOrder(sortOrder === "asc" ? "desc" : "asc");
    } else {
      setSortKey(key);
      setSortOrder("desc");
    }
  };

  const sortedThreadList = useMemo(() => {
    if (!sortKey) return threadList;

    const sorted = [...threadList].sort((a, b) => {
      let compareValue = 0;

      switch (sortKey) {
        case "responseCount":
          compareValue = a.responseCount - b.responseCount;
          break;
        case "speed":
          compareValue =
            calculateThreadSpeed(a.id, a.responseCount, currentTime) -
            calculateThreadSpeed(b.id, b.responseCount, currentTime);
          break;
        case "creationTime":
          compareValue = a.id - b.id;
          break;
        case "lastUpdated":
          compareValue = threadList.indexOf(a) - threadList.indexOf(b);
          break;
      }

      return sortOrder === "asc" ? compareValue : -compareValue;
    });

    return sorted;
  }, [threadList, sortKey, sortOrder, currentTime]);

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
        <h1 className="text-2xl lg:text-4xl grow truncate">
          {
            boards?.find(
              (board: { board_key: string }) =>
                board.board_key === params.boardKey
            )?.name
          }
        </h1>
        <button
          type="button"
          onClick={() => setShowSortControls(!showSortControls)}
          className={twMerge(
            "px-3 py-2 lg:px-4 lg:py-2 mx-1 lg:mx-2 text-sm lg:text-base rounded-md border transition-colors flex items-center gap-1.5",
            sortKey
              ? "bg-blue-600 text-white border-blue-600"
              : "bg-white text-gray-700 border-gray-300 hover:bg-gray-100"
          )}
        >
          <FaSort className="w-4 h-4" />
          <span className="hidden lg:inline">
            {sortKey
              ? `${
                  sortKey === "responseCount"
                    ? "レス数"
                    : sortKey === "speed"
                    ? "勢い"
                    : sortKey === "creationTime"
                    ? "作成"
                    : "更新"
                }${sortOrder === "asc" ? "↑" : "↓"}`
              : "ソート順"}
          </span>
        </button>
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

      {showSortControls && (
        <div className="bg-white border-b border-gray-300 p-3 flex flex-wrap gap-2 items-center">
          <button
            type="button"
            onClick={() => handleSort("lastUpdated")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "lastUpdated"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white text-gray-700 border-gray-300 hover:bg-gray-100"
            )}
          >
            <span>更新日時</span>
            {sortKey === "lastUpdated" &&
              (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "lastUpdated" && <FaSort className="opacity-30" />}
          </button>
          <button
            type="button"
            onClick={() => handleSort("responseCount")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "responseCount"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white text-gray-700 border-gray-300 hover:bg-gray-100"
            )}
          >
            <span>レス数</span>
            {sortKey === "responseCount" &&
              (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "responseCount" && <FaSort className="opacity-30" />}
          </button>
          <button
            type="button"
            onClick={() => handleSort("speed")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "speed"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white text-gray-700 border-gray-300 hover:bg-gray-100"
            )}
          >
            <span>勢い</span>
            {sortKey === "speed" &&
              (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "speed" && <FaSort className="opacity-30" />}
          </button>
          <button
            type="button"
            onClick={() => handleSort("creationTime")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "creationTime"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white text-gray-700 border-gray-300 hover:bg-gray-100"
            )}
          >
            <span>作成日時</span>
            {sortKey === "creationTime" &&
              (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "creationTime" && <FaSort className="opacity-30" />}
          </button>
          {sortKey && (
            <button
              type="button"
              onClick={() => {
                setSortKey(null);
              }}
              className="px-3 py-1.5 text-sm rounded-md border border-gray-300 bg-gray-100 text-gray-700 hover:bg-gray-200 transition-colors"
            >
              リセット
            </button>
          )}
        </div>
      )}

      <div className="flex flex-col lg:grow">
        {sortedThreadList.map((thread, i) => (
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
              <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                <span>{convertLinuxTimeToDateString(thread.id)}</span>
                {thread.authorId && <span>ID:{thread.authorId}</span>}
                <span className="text-blue-600 font-semibold">
                  {calculateThreadSpeed(
                    thread.id,
                    thread.responseCount,
                    currentTime
                  )}
                  /day
                </span>
              </div>
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ThreadListPage;
