import ThreadList from "../components/ThreadList";
import { Link, useParams } from "@remix-run/react";
import Breadcrumb from "../components/Breadcrumb";
import Tab from "../components/Tab";
import { getArchivedThreads, getBoard, getThreads } from "../hooks/queries";
import BoardSetting from "~/components/BoardSetting";

type TabKeys = "threads" | "settings";

const Page = () => {
  const params = useParams();
  if (!params.boardKey) {
    throw new Error("Page not found");
  }

  const { data: board, refetch } = getBoard({
    params: {
      path: {
        board_key: params.boardKey,
      },
    },
  });

  const { data: threads } = getThreads({
    params: {
      path: {
        board_key: params.boardKey,
      },
    },
  });

  const { data: archivedThreads } = getArchivedThreads({
    params: {
      path: {
        board_key: params.boardKey,
      },
    },
  });

  return (
    <div className="p-4">
      <h1 className="text-3xl font-bold">Threads: {params.boardKey}</h1>
      <Breadcrumb>
        <Link
          to="/dashboard/boards"
          className="text-gray-500 hover:text-gray-700"
        >
          Boards
        </Link>
        <span className="text-gray-500" aria-current="page">
          Threads: {board!.name} ({params.boardKey})
        </span>
      </Breadcrumb>
      <Tab
        tabItems={[
          {
            tabKey: "threads",
            tabLabel: "Threads",
            id: "threads-tab",
            children: (
              <div className="p-2">
                <ThreadList
                  threads={
                    threads!.map((x) => ({
                      threadNumber: Number(x.thread_number),
                      title: x.title,
                      responseCount: Number(x.response_count),
                      lastModified: x.last_modified,
                      boardId: Number(board!.id),
                    })) ?? []
                  }
                  board={{
                    boardKey: params.boardKey,
                    boardName: board!.name,
                  }}
                />
              </div>
            ),
          },
          {
            tabKey: "archived-threads",
            tabLabel: "Archives",
            id: "archived-threads-tab",
            children: (
              <div className="p-2">
                <ThreadList
                  threads={
                    archivedThreads!.map((x) => ({
                      threadNumber: Number(x.thread_number),
                      title: x.title,
                      responseCount: Number(x.response_count),
                      lastModified: x.last_modified,
                      boardId: Number(board!.id),
                    })) ?? []
                  }
                  board={{
                    boardKey: params.boardKey,
                    boardName: board!.name,
                  }}
                  archives
                />
              </div>
            ),
          },
          {
            tabKey: "settings",
            tabLabel: "Settings",
            id: "settings-tab",
            children: <BoardSetting board={board!} refetchBoard={refetch} />,
          },
        ]}
      />
    </div>
  );
};

export default Page;
