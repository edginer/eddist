import { useSuspenseQuery } from "@tanstack/react-query";
import { Button, HR } from "flowbite-react";
import { useState } from "react";
import { FaArrowLeft } from "react-icons/fa";
import { Link, useNavigate, useParams } from "react-router";
import { twMerge } from "tailwind-merge";
import PostThreadModal from "../PostThreadModal";

interface Thread {
  title: string;
  id: number;
  responseCount: number;
  authorId?: string;
}

const convertSubjectTextToThreadList = (text: string): Thread[] => {
  const lines = text.split("\n");
  const threadList = lines
    .map((line) => {
      const lineRegex = /^(\d{9,10}\.dat)<>(.*) \((\d{1,5})\)$/;
      const lineRegexWithId =
        /^(\d{9,10}\.dat)<>(.*) \[(.{4,13})★\] \((\d{1,5})\)$/;
      const match = line.match(lineRegexWithId);
      if (match == null) {
        const match2 = line.match(lineRegex);
        if (match2 == null) {
          return undefined;
        }

        const id = parseInt(match2[1].split(".")[0]);
        const title = match2[2];
        const responseCount = parseInt(match2[3]);

        return {
          title,
          id,
          responseCount,
          authorId: undefined,
        };
      }
      const id = parseInt(match[1].split(".")[0]);
      const title = match[2];
      const authorId = match[3];
      const responseCount = parseInt(match[4]);

      return {
        title,
        id,
        responseCount,
        authorId,
      };
    })
    .filter((thread) => thread != null) as Thread[];
  return threadList;
};

const convertLinuxTimeToDateString = (linuxTime: number): string => {
  const dateTime = new Date(linuxTime * 1000);

  const datetimeStr = dateTime.toLocaleString("ja-JP", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
  return datetimeStr;
};

const ThreadListPage = () => {
  const params = useParams();
  const navigate = useNavigate();

  const { data: boards } = useSuspenseQuery({
    queryKey: ["boards"],
    queryFn: () => fetch("/api/boards").then((res) => res.json()),
  });

  const { data, refetch } = useSuspenseQuery({
    queryKey: ["threadList", params.boardKey],
    queryFn: async () => {
      const res = await fetch(`/${params.boardKey}/subject.txt`, {
        headers: {
          "Content-Type": "text/plain; charset=shift_jis",
          // "X-ThreadList-AuthorId-Supported": "true",
        },
      });
      const sjisText = await res.blob();
      const arrayBuffer = await sjisText.arrayBuffer();
      const text = new TextDecoder("shift_jis").decode(arrayBuffer);

      return convertSubjectTextToThreadList(text);
    },
  });

  const [creatingThread, setCreatingThread] = useState(false);

  return (
    <div>
      <PostThreadModal
        boardKey={params.boardKey!}
        open={creatingThread}
        setOpen={setCreatingThread}
        refetchThreadList={refetch}
      />
      <header className="flex justify-between items-center">
        <Link to="/">
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
          onClick={() => setCreatingThread(true)}
          className={twMerge("px-2 mx-4", params.boardKey || "hidden")}
        >
          スレッド作成
        </Button>
      </header>
      <HR className="my-4" />
      <div className="flex flex-col lg:flex-grow">
        {data.map((thread, i) => (
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
