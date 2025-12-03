import { FaPlus } from "react-icons/fa";
import BoardItem from "../components/BoardItem";
import { getBoards } from "~/hooks/queries";
import { useState } from "react";
import CreateBoardModal from "~/components/CreateBoardModal";

function Page() {
  const { data: boards, refetch } = getBoards({});
  const [openCreateBoardModal, setOpenCreateBoardModal] = useState(false);

  return (
    <>
      <CreateBoardModal
        open={openCreateBoardModal}
        setOpen={setOpenCreateBoardModal}
        refetch={refetch}
      />
      <div className="p-4">
        <div className="flex">
          <h1 className="text-3xl font-bold grow">Boards</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => setOpenCreateBoardModal(true)}
          >
            <FaPlus />
          </button>
        </div>

        <div className="grid grid-cols-1 p-1 pt-6 gap-4">
          {boards!.map((board) => (
            <BoardItem
              key={board.id}
              boardKey={board.board_key}
              boardName={board.name}
              threadCount={board.thread_count}
            />
          ))}
        </div>
      </div>
    </>
  );
}

export default Page;
