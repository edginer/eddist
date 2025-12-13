import { Button } from "flowbite-react";
import { useState, useEffect, useMemo } from "react";
import { Link, useParams } from "react-router";
import { FaArrowLeft, FaCog, FaPen, FaSync } from "react-icons/fa";
import { twMerge } from "tailwind-merge";
import PostResponseModal from "../components/PostResponseModal";
import { NGWordsSettingsModal } from "../components/NGWordsSettingsModal";
import { NGContextMenu } from "../components/NGContextMenu";
import { FloatingNGButton } from "../components/FloatingNGButton";
import type { Route } from "./+types/ThreadPage";
import useSWR from "swr";
import { fetchBoards, type Board } from "~/api-client/board";
import {
  fetchThread,
  type BodyAnchorPart,
  type Response,
} from "~/api-client/thread";
import { useNGWords } from "~/contexts/NGWordsContext";
import { useContextMenu } from "~/hooks/useContextMenu";
import { usePullToRefresh } from "~/hooks/usePullToRefresh";
import React from "react";

export const headers = (_: Route.HeadersArgs) => {
  return {
    "X-Frame-Options": "DENY",
    "X-Content-Type-Options": "nosniff",
    "Cache-Control": "max-age=15, s-maxage=30",
  };
};

export const loader = async ({ params, context }: Route.LoaderArgs) => {
  if (!params.boardKey || !params.threadKey) {
    throw new Error("Invalid parameters");
  }

  const boardKeyRegex = /^[a-z0-9\-_]{1,63}$/;
  if (!boardKeyRegex.test(params.boardKey)) {
    throw new Response("Not Found", { status: 404 });
  }

  const threadKeyRegex = /^\d{10}$/;
  if (!threadKeyRegex.test(params.threadKey)) {
    throw new Response("Not Found", { status: 404 });
  }

  const [thread, boards] = await Promise.all([
    fetchThread(params.boardKey!, params.threadKey!, {
      baseUrl:
        context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL,
    }),
    fetchBoards({
      baseUrl:
        context.EDDIST_SERVER_URL ?? import.meta.env.VITE_EDDIST_SERVER_URL,
    }),
  ]);

  return {
    thread,
    boards,
    eddistData: {
      bbsName: context.BBS_NAME ?? "エッヂ掲示板",
      availableUserRegistration: context.ENABLE_USER_REGISTRATION ?? false,
    },
  } satisfies {
    thread: { threadName: string; responses: Response[] };
    boards: Board[];
    eddistData: {
      bbsName: string;
      availableUserRegistration: boolean;
    };
  };
};

const lastResponseDate = (responses: Response[]): Date | undefined => {
  if (responses.length === 0) return undefined;
  const lastResponse = responses[responses.length - 1];
  // Date format is 2025/06/01(日) 15:36:48.602
  const dateParts = lastResponse.date.split(" ");
  const dateStr = dateParts[0].replace(/\(.+\)/, ""); // Remove day of the week
  const timeStr = dateParts[1].split(".")[0]; // Remove milliseconds
  const fullDateStr = `${dateStr} ${timeStr}`;
  return new Date(fullDateStr);
};

const diffFromNow = (date: Date): number => {
  const now = new Date();
  return Math.floor((now.getTime() - date.getTime()) / 1000); // Return difference in seconds
};

interface Popup {
  id: number;
  x: number;
  y: number;
  posts: Response[];
  ref: React.RefObject<HTMLDivElement | null>;
}

let popupCounter = 0;

// Set maximum width and height for popups
const MAX_POPUP_WIDTH_DESKTOP = "90vw";
const MAX_POPUP_WIDTH_MOBILE = "95vw";
const MAX_POPUP_HEIGHT_MOBILE = "calc(90vh - 50px)";
const MOBILE_BREAKPOINT = 768; // Tailwind's default mobile breakpoint

const Meta = ({
  bbsName,
  threadName,
}: {
  bbsName: string;
  threadName: string;
}) => (
  <>
    <title>{`${threadName} - ${bbsName}`}</title>
    <meta property="og:title" content={`${bbsName} | ${threadName}`} />
    <meta property="og:site_name" content={bbsName} />
    <meta property="og:type" content="website" />
    <meta name="twitter:title" content={`${bbsName} | ${threadName}`} />
  </>
);

const ThreadPage = ({
  loaderData: { boards, thread, eddistData },
}: Route.ComponentProps) => {
  const params = useParams();

  const [popups, setPopups] = useState<Popup[]>([]);

  useEffect(() => {
    const htmlEl = document.documentElement;
    const originalOverflow = htmlEl.style.overflow;
    const originalPaddingRight = htmlEl.style.paddingRight;

    if (popups.length > 0) {
      const scrollbarWidth = window.innerWidth - htmlEl.clientWidth;
      if (scrollbarWidth > 0) {
        htmlEl.style.paddingRight = `${scrollbarWidth}px`;
      }
      htmlEl.style.overflow = "hidden";
    } else {
      htmlEl.style.overflow = originalOverflow;
      htmlEl.style.paddingRight = originalPaddingRight;
    }

    return () => {
      htmlEl.style.overflow = originalOverflow;
      htmlEl.style.paddingRight = originalPaddingRight;
    };
  }, [popups.length]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popups.length === 0) return;
      const top = popups[popups.length - 1];
      if (top.ref.current && !top.ref.current.contains(e.target as Node)) {
        setPopups((p) => p.slice(0, -1));
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [popups]);

  const openPopup = (e: React.MouseEvent, indices: number[]) => {
    if (indices.length === 0) return;

    let x: number, y: number;
    const isMobile = window.innerWidth < MOBILE_BREAKPOINT;

    if (isMobile) {
      x = window.innerWidth * 0.025;
      y = window.innerHeight * 0.05;
    } else {
      x = Math.min(e.clientX - 10, window.innerWidth - 300);
      y = Math.min(e.clientY - 10, window.innerHeight - 300);
    }

    const ref = React.createRef<HTMLDivElement>();
    const postsByIndices = indices
      .filter((i) => i >= 0 && i < posts.responses.length)
      .map((i) => posts.responses[i]);
    setPopups((prev) => [
      ...prev,
      { id: ++popupCounter, x, y, posts: postsByIndices, ref },
    ]);
  };

  const [creatingResponse, setCreatingResponse] = useState(false);
  const [showNGSettings, setShowNGSettings] = useState(false);
  const [expandedNGPosts, setExpandedNGPosts] = useState<Set<number>>(
    new Set()
  );
  const [isRefreshing, setIsRefreshing] = useState(false);

  const { shouldFilterResponse } = useNGWords();
  const { menuState, closeMenu, contextMenuHandlers } = useContextMenu();
  const [contextMenuResponse, setContextMenuResponse] =
    useState<Response | null>(null);
  const [contextMenuType, setContextMenuType] = useState<
    "authorId" | "name" | null
  >(null);

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
    direction: "up",
    enabled: true,
    scrollTarget: "window",
  });

  const { data: posts, mutate } = useSWR(
    `${params.boardKey}/dat/${params.threadKey}.dat`,
    () => fetchThread(params.boardKey!, params.threadKey!),
    {
      fallbackData: thread,
      revalidateOnMount: false,
    }
  );

  // Find current board
  const currentBoard = boards?.find(
    (board: { board_key: string }) => board.board_key === params.boardKey
  );

  const boardName = currentBoard?.name || "";
  const threadName = useMemo(
    () => decodeNumericCharRefsStr(posts?.threadName || ""),
    [posts?.threadName]
  );

  if (
    thread.redirected &&
    diffFromNow(lastResponseDate(posts?.responses || []) || new Date(0)) >
      60 * 60 * 24 * 1
  ) {
    return (
      <div className="flex flex-col items-center justify-center h-screen">
        <p className="text-gray-600">
          このスレッドはしばらく前にdat落ちしたため、Webブラウザからは閲覧できません。
        </p>
        <p className="text-gray-600">
          専用ブラウザなどを使用して閲覧してください。
        </p>
        <Link to={`/${params.boardKey}`} className="mt-4 text-blue-500">
          戻る
        </Link>
      </div>
    );
  }

  return (
    <div className={twMerge("flex flex-col")}>
      {/* Pull-to-refresh indicator at bottom */}
      {(isPulling || isPullRefreshing) && (
        <div
          className="fixed bottom-0 left-0 right-0 z-40 flex justify-center items-center transition-all duration-200"
          style={{
            paddingBottom: isPullRefreshing
              ? "16px"
              : `${Math.min(pullDistance / 2, 40)}px`,
          }}
        >
          <div
            className="bg-white rounded-full shadow-lg flex items-center justify-center transition-all duration-200"
            style={{
              width: "40px",
              height: "40px",
              transform: `scale(${isPullRefreshing ? 1 : Math.min(pullDistance / 80, 1)})`,
              opacity: isPullRefreshing ? 1 : Math.min(pullDistance / 80, 0.8),
            }}
          >
            <FaSync
              className={twMerge(
                "text-blue-600 text-lg",
                isPullRefreshing && "animate-spin"
              )}
            />
          </div>
        </div>
      )}

      <header className="fixed top-0 left-0 right-0 z-5 bg-white shadow-md transition-transform duration-300 flex items-center p-3 lg:p-4 h-18 lg:h-16">
        <Link to={`/${params.boardKey}`}>
          <FaArrowLeft className="mr-1 lg:mx-2 lg:mr-4 w-6 h-6" />
        </Link>

        <>
          <Meta bbsName={eddistData?.bbsName} threadName={threadName} />
          {/* Mobile header - Board name above thread name */}
          <div className="grow md:hidden">
            <p className="text-xs text-gray-600 truncate">{boardName}</p>
            <h1
              className="text-sm line-clamp-2 font-medium break-all"
              title={threadName}
              dangerouslySetInnerHTML={{ __html: threadName }}
            ></h1>
          </div>

          {/* Desktop header - Board name and thread name on same line */}
          <div className="hidden md:flex items-center grow">
            <h1 className="text-2xl whitespace-nowrap" title={boardName}>
              {boardName}
            </h1>
            {threadName && (
              <>
                <span className="mx-3 ml-4">-</span>
                <p
                  className="text-xl line-clamp-2 break-all"
                  title={threadName}
                  dangerouslySetInnerHTML={{ __html: threadName }}
                ></p>
              </>
            )}
          </div>
        </>
        <button
          type="button"
          onClick={handleRefresh}
          disabled={isRefreshing || isPullRefreshing}
          className="hidden lg:flex px-3 py-2 lg:px-4 lg:py-2 mx-1 text-sm lg:text-base rounded-md border border-gray-300 bg-white text-gray-700 hover:bg-gray-100 transition-colors items-center disabled:opacity-50 disabled:cursor-not-allowed"
          title="更新"
        >
          <FaSync
            className={twMerge(
              "w-4 h-4",
              (isRefreshing || isPullRefreshing) && "animate-spin"
            )}
          />
        </button>
        <button
          type="button"
          onClick={() => setShowNGSettings(true)}
          className="px-3 py-2 lg:px-4 lg:py-2 mx-1 text-sm lg:text-base rounded-md border border-gray-300 bg-white text-gray-700 hover:bg-gray-100 transition-colors"
          title="NG設定"
        >
          <FaCog className="w-4 h-4" />
        </button>
        <Button
          onClick={() => setCreatingResponse(true)}
          className={twMerge(
            "px-3 py-2 lg:px-6 lg:py-3 lg:mx-2 w-12 h-10 lg:w-35",
            params.boardKey || params.threadKey || "hidden"
          )}
        >
          <FaPen className="lg:mr-3" />
          <span className="lg:block hidden">書き込み</span>
        </Button>

        <NGWordsSettingsModal
          open={showNGSettings}
          setOpen={setShowNGSettings}
        />
      </header>

      <PostResponseModal
        open={creatingResponse}
        setOpen={setCreatingResponse}
        boardKey={params.boardKey!}
        threadKey={params.threadKey!}
        refetchThread={mutate}
      />

      <main className="grow pt-18 lg:pt-16">
        <div className="max-w-7xl mx-auto">
          <div className="bg-white border border-gray-300 rounded-lg shadow-md">
            {posts?.responses.map((post) => {
              const filterResult = shouldFilterResponse(post);
              const isExpanded = expandedNGPosts.has(post.id);

              // Completely hidden
              if (filterResult.filtered && filterResult.hideMode === "hidden") {
                return null;
              }

              // Collapsed with expand option
              if (
                filterResult.filtered &&
                filterResult.hideMode === "collapsed" &&
                !isExpanded
              ) {
                return (
                  <div
                    key={post.id}
                    className="border-b border-gray-300 p-4 bg-gray-50"
                  >
                    <div className="text-sm text-gray-400 flex justify-between items-center">
                      <span>{post.id}. このレスはNG設定により非表示</span>
                      <button
                        onClick={() =>
                          setExpandedNGPosts((prev) => {
                            const newSet = new Set(prev);
                            newSet.add(post.id);
                            return newSet;
                          })
                        }
                        className="text-blue-500 hover:text-blue-700 text-xs px-2 py-1 rounded hover:bg-blue-50 transition-colors"
                        type="button"
                      >
                        表示
                      </button>
                    </div>
                  </div>
                );
              }

              // Normal rendering (not filtered or expanded)
              return (
                <div key={post.id} className="border-b border-gray-300 p-4">
                  <div className="text-sm text-gray-500">
                    {post.id}{" "}
                    {post.refs && constructReferredNum(post.refs, openPopup)}.{" "}
                    <span
                      className="select-none md:select-auto"
                      onContextMenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setContextMenuResponse(post);
                        setContextMenuType("name");
                        contextMenuHandlers.onContextMenu(e);
                      }}
                      onTouchStart={(e) => {
                        setContextMenuResponse(post);
                        setContextMenuType("name");
                        contextMenuHandlers.onTouchStart(e);
                      }}
                      onTouchMove={contextMenuHandlers.onTouchMove}
                      onTouchEnd={contextMenuHandlers.onTouchEnd}
                      onTouchCancel={contextMenuHandlers.onTouchCancel}
                    >
                      {processPostName(post.name)}
                    </span>{" "}
                    {post.date}{" "}
                    <span
                      onClick={(e) =>
                        openPopup(
                          e,
                          posts.responses.reduce(
                            (acc, cur, i) =>
                              cur.authorId === post.authorId
                                ? acc.concat(i)
                                : acc,
                            [] as number[]
                          )
                        )
                      }
                      onContextMenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setContextMenuResponse(post);
                        setContextMenuType("authorId");
                        contextMenuHandlers.onContextMenu(e);
                      }}
                      onTouchStart={(e) => {
                        setContextMenuResponse(post);
                        setContextMenuType("authorId");
                        contextMenuHandlers.onTouchStart(e);
                      }}
                      onTouchMove={contextMenuHandlers.onTouchMove}
                      onTouchEnd={contextMenuHandlers.onTouchEnd}
                      onTouchCancel={contextMenuHandlers.onTouchCancel}
                      style={{ cursor: "pointer" }}
                      className={`select-none md:select-auto ${authorIdResponseCountToColor(
                        posts.authorIdMap.get(post.authorId)?.length ?? 0
                      )}`}
                    >
                      ID:{post.authorId}{" "}
                      {(posts.authorIdMap.get(post.authorId)?.length ?? 0) >
                        1 && (
                        <span>
                          ({post.authorIdAppearBeforeCount}/
                          {posts.authorIdMap.get(post.authorId)?.length})
                        </span>
                      )}
                    </span>
                  </div>
                  <div
                    className="text-gray-800 mt-2 wrap-break-word"
                    data-ng-target="body"
                    data-ng-response-id={post.id}
                  >
                    {processPostBody(post.bodyParts, openPopup)}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </main>
      {popups.map((popup, idx) => {
        const isTop = idx === popups.length - 1;
        const isMobile = window.innerWidth < MOBILE_BREAKPOINT;

        return (
          <div
            key={popup.id}
            ref={popup.ref}
            style={{
              top: isMobile ? popup.y + 48 : popup.y,
              left: popup.x,
              maxHeight: isMobile
                ? MAX_POPUP_HEIGHT_MOBILE
                : Math.min(
                    window.innerHeight * 0.9,
                    window.innerHeight - popup.y - 20
                  ),
              zIndex: 1 + idx,
            }}
            className={twMerge(
              "bg-white border border-gray-300 rounded-lg shadow-md p-2 fixed whitespace-normal",
              isMobile
                ? `max-w-[${MAX_POPUP_WIDTH_MOBILE}px] w-[${MAX_POPUP_WIDTH_MOBILE}px]`
                : `max-w-[${MAX_POPUP_WIDTH_DESKTOP}px] w-auto`,
              isTop
                ? "overflow-y-auto pointer-events-auto"
                : "overflow-y-hidden pointer-events-none"
            )}
          >
            {popup.posts.map((p) => (
              <div
                key={p.id}
                className="border-b border-gray-200 mb-2 pb-2 last:border-none last:mb-0 last:pb-0"
              >
                <div className="text-sm text-gray-500">
                  {p.id}. {p.name} {p.date}{" "}
                  <span
                    onClick={(e) =>
                      openPopup(
                        e,
                        posts.responses.reduce(
                          (acc, cur, i) =>
                            cur.authorId === p.authorId ? acc.concat(i) : acc,
                          [] as number[]
                        )
                      )
                    }
                    style={{ cursor: "pointer" }}
                    className={authorIdResponseCountToColor(
                      posts.authorIdMap.get(p.authorId)?.length ?? 0
                    )}
                  >
                    ID:{p.authorId}{" "}
                    {(posts.authorIdMap.get(p.authorId)?.length ?? 0) > 1 && (
                      <span>
                        ({p.authorIdAppearBeforeCount}/
                        {posts.authorIdMap.get(p.authorId)?.length})
                      </span>
                    )}
                  </span>
                </div>
                <div className="text-gray-800 mt-1 wrap-break-word">
                  {processPostBody(p.bodyParts, openPopup)}
                </div>
              </div>
            ))}
          </div>
        );
      })}

      {menuState.isOpen && contextMenuResponse && contextMenuType && (
        <NGContextMenu
          x={menuState.x}
          y={menuState.y}
          onClose={closeMenu}
          options={(() => {
            const bodyText = contextMenuResponse.bodyParts
              .map((part) => part.text)
              .join("");

            if (contextMenuType === "authorId") {
              return [
                {
                  label: "投稿者ID",
                  value: contextMenuResponse.authorId,
                  category: "response.authorIds" as const,
                  isResponse: true,
                },
              ];
            } else if (contextMenuType === "name") {
              return [
                {
                  label: "投稿者名",
                  value: contextMenuResponse.name,
                  category: "response.names" as const,
                  isResponse: true,
                },
              ];
            }
            return [];
          })()}
        />
      )}

      <FloatingNGButton />
    </div>
  );
};

const processPostName = (name: string) => {
  // </b>(Lv1 xxxx-xxxx)<b> to <b>Lv1 xxxx-xxxx</b>
  const regex = /<\/b>(.*?)<b>/g;
  const parts = name.split(regex);
  const processedParts = parts.map((part, index) => {
    if (index % 2 === 0) {
      return part;
    } else {
      return (
        <span key={index} className=" text-cyan-700">
          {part}
        </span>
      );
    }
  });
  return processedParts;
};

const authorIdResponseCountToColor = (
  authorIdResponseCount: number
): string => {
  if (authorIdResponseCount <= 1) {
    return "";
  } else if (authorIdResponseCount <= 5) {
    return "text-blue-500";
  } else {
    return "text-red-500";
  }
};

const constructReferredNum = (
  refs: number[],
  popup: (e: React.MouseEvent, indices: number[]) => void
) => (
  <span
    onClick={(e) =>
      popup(
        e,
        refs.map((x) => x - 1)
      )
    }
    className="text-blue-400 hover:text-blue-600 cursor-pointer"
  >
    (+{refs.length})
  </span>
);

const processPostBody = (
  bodyParts: BodyAnchorPart[],
  popup: (e: React.MouseEvent, indices: number[]) => void
) => (
  <span>
    {bodyParts.map((part, i) =>
      part.isMatch ? (
        <span
          key={i}
          className="text-blue-400 hover:text-blue-600 cursor-pointer"
          onClick={(e) => popup(e, [parseInt(part.text.slice(2)) - 1])}
        >
          {part.text}
        </span>
      ) : (
        <span key={i} dangerouslySetInnerHTML={{ __html: part.text }}></span>
      )
    )}
  </span>
);

const decodeNumericCharRefsStr = (str: string) =>
  str.replace(/&#(x?)([0-9a-fA-F]+);/g, (_, hex, code) => {
    const charCode = hex ? parseInt(code, 16) : parseInt(code, 10);
    return String.fromCodePoint(charCode);
  });

export default ThreadPage;
