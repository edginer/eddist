import { Link } from "react-router";

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

const ThreadList: React.FC<ThreadListProps> = ({ threads, board, archives: isArchives }) => {
  return (
    <div className="divide-y divide-gray-200 rounded border border-gray-200 bg-white">
      {threads.map((thread) => (
        <div
          key={thread.threadNumber}
          className="flex flex-wrap items-start gap-2 p-3 sm:items-center"
        >
          <input type="checkbox" className="mt-1 shrink-0 sm:mt-0" />
          <Link
            to={`/dashboard/boards/${board.boardKey}/${
              isArchives ? "archives" : "threads"
            }/${thread.threadNumber}`}
            className="min-w-0 flex-1 break-words text-blue-500 hover:underline"
          >
            <span>{thread.title}</span>
          </Link>
          <span className="basis-full pl-6 text-sm text-gray-500 sm:ml-auto sm:basis-auto sm:pl-0">
            {thread.responseCount} responses
          </span>
        </div>
      ))}
    </div>
  );
};

export default ThreadList;
