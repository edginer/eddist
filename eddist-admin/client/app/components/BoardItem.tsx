import React from "react";
import { useNavigate } from "react-router";

interface BoardItemProps {
  boardKey: string;
  boardName: string;
  threadCount: number;
}

const BoardItem: React.FC<BoardItemProps> = ({
  boardKey,
  boardName,
  threadCount,
}) => {
  const navigate = useNavigate();

  return (
    <div
      className="rounded-lg mx-4 m-2 bg-white cursor-pointer hover:shadow-md border border-black"
      onClick={() => {
        navigate(`/dashboard/boards/${boardKey}`);
      }}
    >
      <div className="text-gray-500 font-bold text-sm my-2 px-3">
        {boardKey}
      </div>
      <div className="text-gray-900 font-bold text-lg pb-3 px-2 pl-5 border-b border-black">
        {boardName}
      </div>
      <div className="text-gray-900 inline-block p-1">
        <span className="pl-2">Current Thread Count: </span>
        {threadCount}
      </div>
    </div>
  );
};

export default BoardItem;
