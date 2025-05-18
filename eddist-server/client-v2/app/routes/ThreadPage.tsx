import { Button } from "flowbite-react";
import { useState, useRef, useLayoutEffect } from "react";
import { Link, useParams } from "react-router";
import { FaArrowLeft, FaPen } from "react-icons/fa";
import { twMerge } from "tailwind-merge";
import PostResponseModal from "../components/PostResponseModal";
import type { Route } from "./+types/ThreadPage";
import useSWR from "swr";
import { fetchBoards, type Board } from "~/api-client/board";
import { fetchThread, type Response } from "~/api-client/thread";

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

const ThreadPage = ({
  loaderData: { boards, thread },
}: Route.ComponentProps) => {
  const params = useParams();

  const [creatingResponse, setCreatingResponse] = useState(false);
  const [selectedAuthorPosts, setSelectedAuthorPosts] = useState<
    Response[] | null
  >(null);
  const [popupPosition, setPopupPosition] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [popupAdjustedPosition, setPopupAdjustedPosition] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const popupRef = useRef<HTMLDivElement | null>(null);

  useLayoutEffect(() => {
    if (popupPosition && popupRef.current) {
      const popupRect = popupRef.current.getBoundingClientRect();
      let x = popupPosition.x;
      let y = popupPosition.y;
      // Ensure popup stays within window bounds with a 10px margin
      if (x + popupRect.width > window.innerWidth - 10) {
        x = window.innerWidth - popupRect.width - 10;
      }
      if (y + popupRect.height > window.innerHeight - 10) {
        y = window.innerHeight - popupRect.height - 10;
      }
      setPopupAdjustedPosition({ x, y });
    }
  }, [popupPosition]);

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
      <header className="fixed top-0 left-0 right-0 z-50 bg-white shadow-md transition-transform duration-300 flex items-center p-3 lg:p-4 h-18 lg:h-16">
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
      />

      <main className={twMerge("flex-grow pt-18 lg:pt-16", "overflow-y-auto")}>
        <div className="max-w-screen-xl mx-auto">
          <div className="bg-white border border-gray-300 rounded-lg shadow-md">
            {posts?.responses.map((post) => (
              <div key={post.id} className="border-b border-gray-300 p-4">
                <div className="text-sm text-gray-500">
                  {post.id}. {processPostName(post.name)} {post.date}{" "}
                  <span
                    onClick={(e) => {
                      const postsByAuthor = posts?.authorIdMap.get(
                        post.authorId
                      );
                      if (postsByAuthor) {
                        // Adjust coordinates: ensure popup appears within window bounds
                        const x = Math.min(
                          e.clientX - 10,
                          window.innerWidth - 300
                        );
                        const y = Math.min(
                          e.clientY - 10,
                          window.innerHeight - 300
                        );
                        setPopupPosition({ x, y });
                        setSelectedAuthorPosts(
                          postsByAuthor.map(([post]) => ({
                            ...post,
                          }))
                        );
                      }
                    }}
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
      <div
        className="fixed inset-0 bg-transparent z-100"
        style={{ touchAction: "none" }}
        hidden={!popupAdjustedPosition || !selectedAuthorPosts}
        onClick={() => {
          setSelectedAuthorPosts(null);
          setPopupPosition(null);
        }}
      >
        <div
          ref={popupRef}
          onClick={(e) => e.stopPropagation()}
          style={{
            position: "absolute",
            left: popupAdjustedPosition?.x,
            top: popupAdjustedPosition?.y,
          }}
          className="bg-white p-4 rounded-md w-11/12 max-w-md max-h-[calc(100vh-40px)] overflow-y-auto border-2 border-gray-500"
        >
          {selectedAuthorPosts &&
            selectedAuthorPosts.map((post) => (
              <div key={post.id} className="border-b border-gray-300 py-2">
                <div className="text-sm">
                  {post.id}. {processPostName(post.name)} {post.date}{" "}
                  <span
                    onClick={(e) => {
                      const postsByAuthor = posts?.authorIdMap.get(
                        post.authorId
                      );
                      if (postsByAuthor) {
                        // Adjust coordinates: ensure popup appears within window bounds
                        const x = Math.min(
                          e.clientX - 10,
                          window.innerWidth - 300
                        );
                        const y = Math.min(
                          e.clientY - 10,
                          window.innerHeight - 300
                        );
                        setPopupPosition({ x, y });
                        setSelectedAuthorPosts(
                          postsByAuthor.map(([post]) => ({
                            ...post,
                          }))
                        );
                      }
                    }}
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
                  className="text-gray-800 break-words"
                  dangerouslySetInnerHTML={{ __html: post.body }}
                ></div>
              </div>
            ))}
        </div>
      </div>
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
