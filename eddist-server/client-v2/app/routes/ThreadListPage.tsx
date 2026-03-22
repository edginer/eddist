import { Button } from "flowbite-react";
import { lazy, Suspense, useMemo, useRef, useState } from "react";
import { FaArrowLeft, FaCog, FaPen, FaSort, FaSortDown, FaSortUp, FaSync } from "react-icons/fa";
import { Link, useParams } from "react-router";
import useSWR from "swr";
import { twMerge } from "tailwind-merge";
import { type Board, fetchBoards } from "~/api-client/board";
import { fetchClientConfig } from "~/api-client/client-config";
import { fetchThreadList, type Thread } from "~/api-client/thread_list";
import { useNGWords } from "~/contexts/NGWordsContext";
import { useContextMenu } from "~/hooks/useContextMenu";
import { usePullToRefresh } from "~/hooks/usePullToRefresh";
import { getSelectedTextInElement } from "~/utils/selection";
import { NGContextMenu } from "../components/NGContextMenu";
import type { Route } from "./+types/ThreadListPage";

const LazyPostThreadModal = lazy(() => import("../components/PostThreadModal"));
const LazyNGWordsSettingsModal = lazy(() =>
  import("../components/NGWordsSettingsModal").then((m) => ({
    default: m.NGWordsSettingsModal,
  })),
);

type SortKey = "responseCount" | "speed" | "creationTime" | "lastUpdated";
type SortOrder = "asc" | "desc";

export const headers = (_: Route.HeadersArgs) => {
  return {
    "X-Frame-Options": "DENY",
    "X-Content-Type-Options": "nosniff",
    "Cache-Control": "max-age=5, s-maxage=3",
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
  currentTime: number,
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
  const baseUrl = context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL;

  const [threadList, boards, clientConfig] = await Promise.all([
    fetchThreadList(params.boardKey!, { baseUrl }),
    fetchBoards({ baseUrl }),
    fetchClientConfig({ baseUrl }).catch(() => ({
      enable_user_registration: false,
    })),
  ]);

  return {
    threadList,
    boards,
    currentTime: Date.now(),
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
      availableUserRegistration: clientConfig.enable_user_registration,
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

const Meta = ({ bbsName, boardName }: { bbsName: string; boardName: string }) => (
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

  const { data: threadList, mutate } = useSWR(
    `${params.boardKey}/subject.txt`,
    () => fetchThreadList(params.boardKey!),
    {
      fallbackData: data,
      revalidateOnMount: false,
    },
  );

  const [creatingThread, setCreatingThread] = useState(false);
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortOrder, setSortOrder] = useState<SortOrder>("desc");
  const [showSortControls, setShowSortControls] = useState(false);
  const [showNGSettings, setShowNGSettings] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const hasEverOpenedThread = useRef(false);
  const hasEverOpenedNGSettings = useRef(false);

  const { shouldFilterThread } = useNGWords();
  const { menuState, closeMenu, contextMenuHandlers } = useContextMenu();
  const [contextMenuThread, setContextMenuThread] = useState<Thread | null>(null);
  const [selectedTitleText, setSelectedTitleText] = useState<string | null>(null);
  const capturedSelectionRef = useRef<string | null>(null);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await mutate();
    } finally {
      setIsRefreshing(false);
    }
  };

  const {
    isPulling,
    pullDistance,
    isRefreshing: isPullRefreshing,
  } = usePullToRefresh({
    onRefresh: handleRefresh,
    threshold: 80,
    direction: "down",
    enabled: true,
    scrollTarget: "window",
  });

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

  const filteredThreadList = useMemo(() => {
    return sortedThreadList.filter((thread) => !shouldFilterThread(thread));
  }, [sortedThreadList, shouldFilterThread]);

  return (
    <div className="relative pt-16 min-h-screen dark:bg-gray-900 dark:text-gray-100">
      {/* Pull-to-refresh indicator */}
      {(isPulling || isPullRefreshing) && (
        <div
          className="fixed top-16 left-0 right-0 z-40 flex justify-center items-center transition-all duration-200"
          style={{
            paddingTop: isPullRefreshing ? "16px" : `${Math.min(pullDistance / 2, 40)}px`,
          }}
        >
          <div
            className="bg-white dark:bg-gray-800 rounded-full shadow-lg flex items-center justify-center transition-all duration-200"
            style={{
              width: "40px",
              height: "40px",
              transform: `scale(${isPullRefreshing ? 1 : Math.min(pullDistance / 80, 1)})`,
              opacity: isPullRefreshing ? 1 : Math.min(pullDistance / 80, 0.8),
            }}
          >
            <FaSync
              className={twMerge("text-blue-600 text-lg", isPullRefreshing && "animate-spin")}
            />
          </div>
        </div>
      )}

      <header
        className={
          "fixed top-0 left-0 right-0 z-50 bg-white dark:bg-gray-900 shadow-md transition-transform duration-300 transform flex justify-between items-center p-3 lg:p-4"
        }
      >
        <Meta
          bbsName={eddistData.bbsName}
          boardName={
            boards?.find((board: { board_key: string }) => board.board_key === params.boardKey)
              ?.name ?? "スレッド一覧"
          }
        />
        <Link to="/">
          <FaArrowLeft className="mx-2 mr-4 w-6 h-6" />
        </Link>
        <h1 className="text-2xl lg:text-4xl grow truncate">
          {
            boards?.find((board: { board_key: string }) => board.board_key === params.boardKey)
              ?.name
          }
        </h1>
        <button
          type="button"
          onClick={handleRefresh}
          disabled={isRefreshing || isPullRefreshing}
          className="hidden lg:flex px-3 py-2 lg:px-4 lg:py-2 mx-1 lg:mx-2 text-sm lg:text-base rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
          title="更新"
        >
          <FaSync
            className={twMerge("w-4 h-4", (isRefreshing || isPullRefreshing) && "animate-spin")}
          />
        </button>
        <button
          type="button"
          onClick={() => {
            hasEverOpenedNGSettings.current = true;
            setShowNGSettings(true);
          }}
          className="px-3 py-2 lg:px-4 lg:py-2 mx-1 lg:mx-2 text-sm lg:text-base rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center gap-1.5"
          title="NG設定"
        >
          <FaCog className="w-4 h-4" />
        </button>
        <button
          type="button"
          onClick={() => setShowSortControls(!showSortControls)}
          className={twMerge(
            "px-3 py-2 lg:px-4 lg:py-2 mx-1 lg:mx-2 text-sm lg:text-base rounded-md border transition-colors flex items-center gap-1.5",
            sortKey
              ? "bg-blue-600 text-white border-blue-600"
              : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700",
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
          onClick={() => {
            hasEverOpenedThread.current = true;
            setCreatingThread(true);
          }}
          className={twMerge("px-4 py-2 lg:px-6 lg:py-3 mx-2", params.boardKey || "hidden")}
        >
          <FaPen className="lg:mr-3" />
          <span className="lg:block hidden">スレッド作成</span>
        </Button>

        {hasEverOpenedNGSettings.current && (
          <Suspense fallback={null}>
            <LazyNGWordsSettingsModal open={showNGSettings} setOpen={setShowNGSettings} />
          </Suspense>
        )}
      </header>

      {hasEverOpenedThread.current && (
        <Suspense fallback={null}>
          <LazyPostThreadModal
            boardKey={params.boardKey!}
            open={creatingThread}
            setOpen={setCreatingThread}
            refetchThreadList={mutate}
          />
        </Suspense>
      )}

      {showSortControls && (
        <div className="bg-white dark:bg-gray-900 border-b border-gray-300 dark:border-gray-700 p-3 flex flex-wrap gap-2 items-center">
          <button
            type="button"
            onClick={() => handleSort("lastUpdated")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "lastUpdated"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700",
            )}
          >
            <span>更新日時</span>
            {sortKey === "lastUpdated" && (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "lastUpdated" && <FaSort className="opacity-30" />}
          </button>
          <button
            type="button"
            onClick={() => handleSort("responseCount")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "responseCount"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700",
            )}
          >
            <span>レス数</span>
            {sortKey === "responseCount" && (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "responseCount" && <FaSort className="opacity-30" />}
          </button>
          <button
            type="button"
            onClick={() => handleSort("speed")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "speed"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700",
            )}
          >
            <span>勢い</span>
            {sortKey === "speed" && (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "speed" && <FaSort className="opacity-30" />}
          </button>
          <button
            type="button"
            onClick={() => handleSort("creationTime")}
            className={twMerge(
              "px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5",
              sortKey === "creationTime"
                ? "bg-blue-600 text-white border-blue-600"
                : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700",
            )}
          >
            <span>作成日時</span>
            {sortKey === "creationTime" && (sortOrder === "asc" ? <FaSortUp /> : <FaSortDown />)}
            {sortKey !== "creationTime" && <FaSort className="opacity-30" />}
          </button>
          {sortKey && (
            <button
              type="button"
              onClick={() => {
                setSortKey(null);
              }}
              className="px-3 py-1.5 text-sm rounded-md border border-gray-300 dark:border-gray-600 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
            >
              リセット
            </button>
          )}
        </div>
      )}

      <div className="flex flex-col lg:grow">
        {filteredThreadList.map((thread, i) => (
          <div key={thread.id} className="block">
            {i !== 0 && (
              <div className="border-b border-gray-400 dark:border-gray-600 lg:border-none lg:pt-2"></div>
            )}
            <Link
              to={`/${params.boardKey}/${thread.id}`}
              className="hover:bg-gray-200 dark:hover:bg-gray-700 cursor-default text-left block w-full bg-gray-100 dark:bg-gray-800 p-2 lg:p-3 select-none md:select-auto"
              data-ng-target="title"
              data-ng-thread-id={thread.id}
              {...contextMenuHandlers}
              onMouseDown={(e) => {
                // Capture selection before it gets cleared by right-click
                if (e.button === 2) {
                  // Right mouse button
                  capturedSelectionRef.current = getSelectedTextInElement(e.currentTarget);
                }
              }}
              onContextMenu={(e) => {
                // Use captured selection from mousedown
                const selectedText = capturedSelectionRef.current;
                capturedSelectionRef.current = null; // Clear after use
                contextMenuHandlers.onContextMenu(e);
                setContextMenuThread(thread);
                setSelectedTitleText(selectedText);
              }}
              onTouchStart={(e) => {
                const selectedText = getSelectedTextInElement(e.currentTarget);
                contextMenuHandlers.onTouchStart(e);
                setContextMenuThread(thread);
                setSelectedTitleText(selectedText);
              }}
            >
              <div>
                <span
                  className="break-all"
                  // biome-ignore lint/security/noDangerouslySetInnerHtml: BBS thread title rendered as HTML
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
                  {calculateThreadSpeed(thread.id, thread.responseCount, currentTime)}
                  /day
                </span>
              </div>
            </Link>
          </div>
        ))}
      </div>

      {menuState.isOpen && contextMenuThread && (
        <NGContextMenu
          x={menuState.x}
          y={menuState.y}
          onClose={closeMenu}
          actions={[
            {
              label: "新しいタブで開く",
              href: `/${params.boardKey}/${contextMenuThread.id}`,
              target: "_blank",
            },
          ]}
          options={[
            {
              label: selectedTitleText ? "選択したテキスト" : "スレッドタイトル",
              value: selectedTitleText || contextMenuThread.title,
              category: "thread.titles",
            },
            ...(contextMenuThread.authorId
              ? [
                  {
                    label: "スレ投稿者ID",
                    value: contextMenuThread.authorId,
                    category: "thread.authorIds" as const,
                  },
                ]
              : []),
          ]}
        />
      )}
    </div>
  );
};

export default ThreadListPage;
