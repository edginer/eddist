import type React from "react";
import { Link } from "react-router";

interface BoardItemProps {
  boardKey: string;
  boardName: string;
  threadCount: number;
}

const BoardItem: React.FC<BoardItemProps> = ({ boardKey, boardName, threadCount }) => {
  return (
    <Link
      to={`/dashboard/boards/${boardKey}`}
      className="rounded-lg border border-gray-200 bg-white shadow-sm transition-shadow hover:shadow-md"
    >
      <div className="my-2 px-3 text-sm font-bold text-gray-500">{boardKey}</div>
      <div className="border-b border-gray-200 px-3 pb-3 text-lg font-bold text-gray-900">
        {boardName}
      </div>
      <div className="inline-block p-3 text-gray-900">
        <span>Current Thread Count: </span>
        {threadCount}
      </div>
    </Link>
  );
};

export default BoardItem;
