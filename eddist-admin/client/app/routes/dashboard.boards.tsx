import { useState } from "react";
import { FaPlus } from "react-icons/fa";
import CreateBoardModal from "~/components/CreateBoardModal";
import { getBoards } from "~/hooks/queries";
import BoardItem from "../components/BoardItem";

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
      <div className="p-4 sm:p-6 lg:p-8">
        <div className="flex items-center gap-3">
          <h1 className="grow text-2xl font-bold sm:text-3xl">Boards</h1>
          <button
            type="button"
            className="min-h-11 min-w-11 shrink-0 rounded-xl bg-slate-400 p-3 shadow-lg hover:bg-slate-500"
            aria-label="Create board"
            onClick={() => setOpenCreateBoardModal(true)}
          >
            <FaPlus />
          </button>
        </div>

        <div className="grid grid-cols-1 p-1 pt-6 gap-4">
          {boards?.map((board) => (
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
