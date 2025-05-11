import { Button } from "flowbite-react";
import { useState } from "react";
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

export const loader = async ({ params }: Route.LoaderArgs) => {
  if (!params.boardKey || !params.threadKey) {
    throw new Error("Invalid parameters");
  }

  const [thread, boards] = await Promise.all([
    fetchThread(params.boardKey!, params.threadKey!),
    fetchBoards(),
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
    <div className="flex flex-col">
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
          <div className="hidden md:flex flex-grow items-center">
            <h1 className="text-2xl truncate" title={boardName}>
              {boardName}
            </h1>
            {threadName && (
              <>
                <span className="mx-2">-</span>
                <p
                  className="text-xl flex-grow"
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
            "px-3 py-2 lg:px-6 lg:py-3 lg:mx-2 w-12 h-10 lg:w-auto",
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

      <main className="flex-grow pt-18 lg:pt-16 overflow-y-auto">
        <div className="max-w-screen-xl mx-auto">
          <div className="bg-white border border-gray-300 rounded-lg shadow-md">
            {posts?.responses.map((post) => (
              <div key={post.id} className="border-b border-gray-300 p-4">
                <div className="text-sm text-gray-500">
                  {post.id}. {processPostName(post.name)} {post.date}{" "}
                  <span
                    className={authorIdResponseCountToColor(
                      posts.authorIdMap.get(post.authorId)?.length ?? 0
                    )}
                  >
                    ID: {post.authorId}{" "}
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
                  className="text-gray-800 mt-2"
                  dangerouslySetInnerHTML={{ __html: post.body }}
                ></div>
              </div>
            ))}
          </div>
        </div>
      </main>
    </div>
  );
};

const processPostName = (name: string) => {
  // </b>(Lv1 xxxx-xxxx)<b> to <b>Lv1 xxxx-xxxx</b> (string to <b className="...">string</b> (React component))
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
