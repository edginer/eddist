import { Button } from "flowbite-react";
import { useState, useEffect } from "react";
import { Link, useParams } from "react-router";
import { FaArrowLeft, FaPen } from "react-icons/fa";
import { twMerge } from "tailwind-merge";
import PostResponseModal from "../components/PostResponseModal";
import type { Route } from "./+types/ThreadPage";
import useSWR from "swr";
import { fetchBoards, type Board } from "~/api-client/board";
import { fetchThread, type Response } from "~/api-client/thread";
import React from "react";

export const headers = (_: Route.HeadersArgs) => {
  return {
    "X-Frame-Options": "DENY",
    "X-Content-Type-Options": "nosniff",
    "Cache-Control": "max-age=15, s-maxage=15",
  };
};

export const loader = async ({ params, context }: Route.LoaderArgs) => {
  if (!params.boardKey || !params.threadKey) {
    throw new Error("Invalid parameters");
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
  } satisfies {
    thread: { threadName: string; responses: Response[] };
    boards: Board[];
  };
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
const MAX_POPUP_HEIGHT_DESKTOP = "90vh";
const MAX_POPUP_WIDTH_MOBILE = "95vw";
const MAX_POPUP_HEIGHT_MOBILE = "calc(90vh - 50px)";
const MOBILE_BREAKPOINT = 768; // Tailwind's default mobile breakpoint

const ThreadPage = ({
  loaderData: { boards, thread },
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

  const openPopup = (e: React.MouseEvent, authorId: string) => {
    const postsByAuthor = posts.authorIdMap.get(authorId)?.map(([p]) => p);
    if (!postsByAuthor) return;

    // use mobile and desktop breakpoints to determine the position of the popup
    let x, y;
    const isMobile = window.innerWidth < MOBILE_BREAKPOINT;

    if (isMobile) {
      x = window.innerWidth * 0.025;
      y = window.innerHeight * 0.05;
    } else {
      x = Math.min(e.clientX - 10, window.innerWidth - 300);
      y = Math.min(e.clientY - 10, window.innerHeight - 300);
    }

    const ref = React.createRef<HTMLDivElement>();
    setPopups((prev) => [
      ...prev,
      { id: ++popupCounter, x, y, posts: postsByAuthor, ref },
    ]);
  };

  const [creatingResponse, setCreatingResponse] = useState(false);

  const { data: posts, mutate } = useSWR(
    `${params.boardKey}/dat/${params.threadKey}.dat`,
    () => fetchThread(params.boardKey!, params.threadKey!),
    {
      fallbackData: thread,
    }
  );

  // Find current board
  const currentBoard = boards?.find(
    (board: { board_key: string }) => board.board_key === params.boardKey
  );

  const boardName = currentBoard?.name || "";
  const threadName = posts?.threadName || "";

  return (
    <div className={twMerge("flex flex-col")}>
      <header className="fixed top-0 left-0 right-0 z-[10000] bg-white shadow-md transition-transform duration-300 flex items-center p-3 lg:p-4 h-18 lg:h-16">
        <Link to={`/${params.boardKey}`}>
          <FaArrowLeft className="mr-1 lg:mx-2 lg:mr-4 w-6 h-6" />
        </Link>

        <>
          {/* Mobile header - Board name above thread name */}
          <div className="flex-grow md:hidden">
            <p className="text-xs text-gray-600 truncate">{boardName}</p>
            <h1
              className="text-sm line-clamp-2 font-medium break-all"
              title={threadName}
              dangerouslySetInnerHTML={{ __html: threadName }}
            ></h1>
          </div>

          {/* Desktop header - Board name and thread name on same line */}
          <div className="hidden md:flex items-center flex-grow">
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
      </header>

      <PostResponseModal
        open={creatingResponse}
        setOpen={setCreatingResponse}
        boardKey={params.boardKey!}
        threadKey={params.threadKey!}
        refetchThread={mutate}
        className="z-[10001]"
      />

      <main className={twMerge("flex-grow pt-18 lg:pt-16", "overflow-y-auto")}>
        <div className="max-w-screen-xl mx-auto">
          <div className="bg-white border border-gray-300 rounded-lg shadow-md">
            {posts?.responses.map((post) => (
              <div key={post.id} className="border-b border-gray-300 p-4">
                <div className="text-sm text-gray-500">
                  {post.id}. {processPostName(post.name)} {post.date}{" "}
                  <span
                    onClick={(e) => openPopup(e, post.authorId)}
                    style={{ cursor: "pointer" }}
                    className={authorIdResponseCountToColor(
                      posts.authorIdMap.get(post.authorId)?.length ?? 0
                    )}
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
                  className="text-gray-800 mt-2 break-words"
                  dangerouslySetInnerHTML={{ __html: post.body }}
                ></div>
              </div>
            ))}
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
              zIndex: 100 + idx,
            }}
            className={twMerge(
              "bg-white border border-gray-300 rounded-lg shadow-md p-2 fixed whitespace-normal",
              isMobile
                ? `max-w-[${MAX_POPUP_WIDTH_MOBILE}px] w-[${MAX_POPUP_WIDTH_MOBILE}px] max-h-[${MAX_POPUP_HEIGHT_MOBILE}px]`
                : `max-w-[${MAX_POPUP_WIDTH_DESKTOP}px] w-auto max-h-[${MAX_POPUP_HEIGHT_DESKTOP}px]`,
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
                    onClick={(e) => openPopup(e, p.authorId)}
                    className="text-blue-600 cursor-pointer"
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
                <div
                  className="text-gray-800 mt-1 break-words"
                  dangerouslySetInnerHTML={{ __html: p.body }}
                />
              </div>
            ))}
          </div>
        );
      })}
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

export default ThreadPage;
