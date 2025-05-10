import { Button, HR } from "flowbite-react";
import { useState } from "react";
import { Link, useParams } from "react-router";
import { FaArrowLeft } from "react-icons/fa";
import { twMerge } from "tailwind-merge";
import PostResponseModal from "../components/PostResponseModal";
import type { Board } from "./TopPage";
import type { Route } from "./+types/ThreadPage";

interface Response {
  name: string;
  mail: string;
  date: string;
  authorId: string;
  body: string;
  id: number;
}

const convertThreadTextToResponseList = (text: string) => {
  const lines = text.split("\n").filter((x) => x !== "");
  let threadTitle = "";
  const responses = lines.map((line, idx) => {
    const lineRegex = /^(.*)<>(.*)<>(.*) ID:(.*)<>(.*)<>(.*)$/;
    const match = line.match(lineRegex);
    if (match == null) {
      // あぼーん<>あぼーん<><> あぼーん<> てす
      const aboneRegex = /^(.*)<>(.*)<><> あぼーん<>(.*)$/;
      const aboneMatch = line.match(aboneRegex);
      if (aboneMatch == null) {
        throw new Error(`Invalid response line: ${line}`);
      }

      if (idx === 0) {
        threadTitle = aboneMatch[3];
      }

      return {
        name: aboneMatch[1],
        mail: "",
        date: "",
        authorId: "",
        body: "あぼーん",
        id: idx + 1,
      };
    }
    const name = match[1];
    const mail = match[2];
    const date = match[3];
    const authorId = match[4];
    const body = match[5];
    if (idx === 0) {
      threadTitle = match[6];
    }

    return {
      name,
      mail,
      date,
      authorId,
      body,
      id: idx + 1,
    };
  });

  return {
    threadName: threadTitle,
    responses: responses satisfies Response[],
  };
};

export const loader = async ({ params }: Route.LoaderArgs) => {
  const threadPromise = async () => {
    const res = await fetch(
      `${import.meta.env.VITE_SSR_BASE_URL}/${params.boardKey}/dat/${
        params.threadKey
      }.dat`,
      {
        headers: {
          "Content-Type": "text/plain; charset=shift_jis",
        },
        redirect: "manual",
      }
    );
    const sjisText = await res.blob();
    const arrayBuffer = await sjisText.arrayBuffer();
    const text = new TextDecoder("shift_jis").decode(arrayBuffer);
    return convertThreadTextToResponseList(text);
  };
  const boardsPromise = async () => {
    return await fetch(`${import.meta.env.VITE_SSR_BASE_URL}/api/boards`).then(
      (res) => res.json() as Promise<Board[]>
    );
  };
  const [thread, boards] = await Promise.all([
    threadPromise(),
    boardsPromise(),
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
  loaderData: { boards, thread: posts },
}: Route.ComponentProps) => {
  const params = useParams();

  const [creatingResponse, setCreatingResponse] = useState(false);

  return (
    <div>
      <PostResponseModal
        open={creatingResponse}
        setOpen={setCreatingResponse}
        boardKey={params.boardKey!}
        threadKey={params.threadKey!}
        refetchThread={async () => {}}
        // refetchThread={refetch}
      />
      <header className="flex justify-between items-center">
        <Link to={`/${params.boardKey}`}>
          <FaArrowLeft className="mx-2 mr-4 w-6 h-6" />
        </Link>
        <h1 className="text-3xl lg:text-5xl flex-grow">
          {
            boards?.find(
              (board: { board_key: string }) =>
                board.board_key === params.boardKey
            )?.name
          }
        </h1>
        <Button
          onClick={() => setCreatingResponse(true)}
          className={twMerge(
            "px-6 py-3 mx-4",
            params.boardKey || params.threadKey || "hidden"
          )}
        >
          書き込み
        </Button>
      </header>
      <HR className="my-4" />
      <div className="mx-auto bg-white border border-gray-300 rounded-lg shadow-md mt-4">
        <div className="p-4 bg-gray-100 border-b border-gray-300">
          <div
            className="text-lg"
            dangerouslySetInnerHTML={{ __html: posts?.threadName ?? "" }}
          ></div>
        </div>
        <div className="overflow-y-auto max-h-[calc(100vh-11rem)] lg:max-h-[calc(100vh-14rem)]">
          {posts?.responses.map((post) => (
            <div key={post.id} className="border-b border-gray-300 p-4">
              <div className="text-sm text-gray-500">
                {post.id}. {post.name} {post.date} ID: {post.authorId}
              </div>
              <div
                className="text-gray-800 mt-2"
                dangerouslySetInnerHTML={{ __html: post.body }}
              ></div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default ThreadPage;
