import { Link } from "@remix-run/react";

interface Board {
  boardKey: string;
  boardName: string;
}

interface Thread {
  threadNumber: number;
  title: string;
  responseCount: number;
  boardId: number;
  lastModified: string;
}

interface ThreadListProps {
  threads: Thread[];
  board: Board;
  archives?: boolean;
}

const ThreadList: React.FC<ThreadListProps> = ({
  threads,
  board,
  archives: isArchives,
}) => {
  return (
    <div className="rounded border border-black divide-y divide-black">
      {threads.map((thread) => (
        <div key={thread.threadNumber} className="flex items-center p-2">
          <input type="checkbox" className="mr-2" />
          <Link
            to={`/dashboard/boards/${board.boardKey}/${
              isArchives ? "archives" : "threads"
            }/${thread.threadNumber}`}
            className="text-blue-500 hover:underline cursor-pointer"
          >
            <span className="flex-grow">{thread.title}</span>
          </Link>
          <span className="ml-auto mr-4">{thread.responseCount} responses</span>
        </div>
      ))}
    </div>
  );
};

export default ThreadList;
