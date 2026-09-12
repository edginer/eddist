import { Link } from "react-router";

interface Board {
  boardKey: string;
  boardName: string;
}

interface Thread {
  threadNumber: number;
  title: string;
  responseCount: number;
  archived: boolean;
}

interface ThreadListProps {
  threads: Thread[];
  board: Board;
  archives?: boolean;
  selection?: {
    selectedThreadNumbers: number[];
    onChange: (threadNumber: number, selected: boolean) => void;
  };
}

const ThreadList: React.FC<ThreadListProps> = ({
  threads,
  board,
  archives: isArchives,
  selection,
}) => {
  return (
    <div className="divide-y divide-gray-200 rounded border border-gray-200 bg-white">
      {threads.length === 0 ? (
        <p className="p-6 text-center text-sm text-gray-500">No threads found.</p>
      ) : (
        threads.map((thread) => {
          const isSelectable = selection != null && !thread.archived;
          const isSelected =
            isSelectable && selection.selectedThreadNumbers.includes(thread.threadNumber);

          return (
            <div
              key={thread.threadNumber}
              className={`flex flex-wrap items-start gap-2 p-3 sm:items-center ${
                isSelected ? "bg-blue-50" : thread.archived ? "bg-gray-50" : ""
              }`}
            >
              {selection && (
                <input
                  type="checkbox"
                  className="mt-1 shrink-0 sm:mt-0"
                  aria-label={`Select thread ${thread.threadNumber}`}
                  checked={isSelected}
                  disabled={!isSelectable}
                  onChange={(event) =>
                    isSelectable && selection.onChange(thread.threadNumber, event.target.checked)
                  }
                />
              )}
              <Link
                to={`/dashboard/boards/${board.boardKey}/${
                  isArchives ? "archives" : "threads"
                }/${thread.threadNumber}`}
                className="min-w-0 flex-1 break-words text-blue-500 hover:underline"
              >
                <span>{thread.title}</span>
              </Link>
              {thread.archived && (
                <span className="shrink-0 rounded bg-gray-200 px-2 py-1 text-xs font-medium text-gray-600">
                  Archived
                </span>
              )}
              <span
                className={`basis-full text-sm text-gray-500 sm:ml-auto sm:basis-auto ${
                  selection ? "pl-6 sm:pl-0" : ""
                }`}
              >
                {thread.responseCount} responses
              </span>
            </div>
          );
        })
      )}
    </div>
  );
};

export default ThreadList;
