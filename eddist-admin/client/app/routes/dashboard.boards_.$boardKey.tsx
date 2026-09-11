import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from "flowbite-react";
import { type FormEvent, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import BoardSetting from "~/components/BoardSetting";
import Breadcrumb from "../components/Breadcrumb";
import Tab from "../components/Tab";
import ThreadList from "../components/ThreadList";
import {
  getBoard,
  getThreads,
  useArchiveThreads,
  useResolveArchivedThread,
} from "../hooks/queries";

interface ThreadListItem {
  threadNumber: number;
  title: string;
  responseCount: number;
  archived: boolean;
}

const ThreadsTabContent = ({
  boardKey,
  threads,
}: {
  boardKey: string;
  threads: ThreadListItem[];
}) => {
  const archiveThreads = useArchiveThreads();
  const [selectedThreadNumbers, setSelectedThreadNumbers] = useState<number[]>([]);
  const [showArchiveConfirm, setShowArchiveConfirm] = useState(false);

  const selectableThreadNumbers = threads
    .filter((thread) => !thread.archived)
    .map((thread) => thread.threadNumber);
  const allSelected =
    selectableThreadNumbers.length > 0 &&
    selectableThreadNumbers.every((threadNumber) => selectedThreadNumbers.includes(threadNumber));

  const handleThreadSelection = (threadNumber: number, selected: boolean) => {
    setSelectedThreadNumbers((current) => {
      if (selected) {
        return current.includes(threadNumber) ? current : [...current, threadNumber];
      }
      return current.filter((currentThreadNumber) => currentThreadNumber !== threadNumber);
    });
  };

  const handleArchive = () => {
    if (selectedThreadNumbers.length === 0) {
      return;
    }

    archiveThreads.mutate(
      {
        params: { path: { board_key: boardKey } },
        body: { thread_numbers: selectedThreadNumbers },
      },
      {
        onSuccess: () => {
          setSelectedThreadNumbers([]);
          setShowArchiveConfirm(false);
        },
      },
    );
  };

  return (
    <>
      <div className="mb-4 flex flex-wrap items-center gap-3 rounded-lg border border-gray-200 bg-white p-3">
        <label
          className="flex items-center gap-2 text-sm text-gray-700"
          htmlFor="select-all-threads"
        >
          <input
            id="select-all-threads"
            type="checkbox"
            checked={allSelected}
            disabled={selectableThreadNumbers.length === 0}
            onChange={(event) =>
              setSelectedThreadNumbers(event.target.checked ? selectableThreadNumbers : [])
            }
          />
          Select all
        </label>
        {selectedThreadNumbers.length > 0 && (
          <span className="text-sm text-gray-500">{selectedThreadNumbers.length} selected</span>
        )}
        <Button
          color="failure"
          disabled={selectedThreadNumbers.length === 0 || archiveThreads.isPending}
          onClick={() => setShowArchiveConfirm(true)}
          className="ml-auto"
        >
          Drop selected threads
        </Button>
      </div>

      <ThreadList
        threads={threads}
        board={{ boardKey, boardName: "" }}
        selection={{
          selectedThreadNumbers,
          onChange: handleThreadSelection,
        }}
      />

      <Modal
        show={showArchiveConfirm}
        onClose={() => {
          if (!archiveThreads.isPending) {
            setShowArchiveConfirm(false);
          }
        }}
        dismissible={!archiveThreads.isPending}
      >
        <ModalHeader>Drop selected threads</ModalHeader>
        <ModalBody>
          <p className="text-gray-700 dark:text-gray-300">
            Drop {selectedThreadNumbers.length} selected thread
            {selectedThreadNumbers.length === 1 ? "" : "s"}?
          </p>
          <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
            Dropped threads will stop appearing on the public thread list and will be archived by
            the scheduled archive job.
          </p>
        </ModalBody>
        <ModalFooter>
          <Button
            color="gray"
            disabled={archiveThreads.isPending}
            onClick={() => setShowArchiveConfirm(false)}
          >
            Cancel
          </Button>
          <Button color="failure" disabled={archiveThreads.isPending} onClick={handleArchive}>
            {archiveThreads.isPending ? "Dropping..." : "Drop threads"}
          </Button>
        </ModalFooter>
      </Modal>
    </>
  );
};

const ArchivedThreadsTabContent = ({ boardKey }: { boardKey: string }) => {
  const navigate = useNavigate();
  const resolveArchivedThread = useResolveArchivedThread();
  const [threadNumber, setThreadNumber] = useState("");
  const [inputError, setInputError] = useState<string>();

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const parsedThreadNumber = Number(threadNumber);
    if (
      threadNumber.trim() === "" ||
      !Number.isSafeInteger(parsedThreadNumber) ||
      parsedThreadNumber < 0
    ) {
      resolveArchivedThread.reset();
      setInputError("Enter a valid thread number.");
      return;
    }

    setInputError(undefined);
    resolveArchivedThread.mutate(
      {
        params: {
          path: {
            board_key: boardKey,
            thread_id: parsedThreadNumber,
          },
        },
      },
      {
        onSuccess: (result) => {
          if (!result) {
            setInputError("Archived thread was not found.");
            return;
          }

          const datSuffix = result.source === "dat" ? "/dat" : "";
          navigate(`/dashboard/boards/${boardKey}/archives/${parsedThreadNumber}${datSuffix}`);
        },
      },
    );
  };

  return (
    <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-700 dark:bg-gray-800">
      <h2 className="mb-4 text-lg font-semibold text-gray-900 dark:text-white">
        Search archived threads
      </h2>
      <form className="flex max-w-lg items-end gap-3" onSubmit={handleSubmit}>
        <div className="flex-1">
          <label
            className="mb-2 block text-sm font-medium text-gray-900 dark:text-white"
            htmlFor="archived-thread-number"
          >
            Thread number
          </label>
          <input
            className="block w-full rounded-lg border border-gray-300 bg-gray-50 p-2.5 text-sm text-gray-900 focus:border-blue-500 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400"
            id="archived-thread-number"
            inputMode="numeric"
            min="0"
            onChange={(event) => setThreadNumber(event.target.value)}
            placeholder="1234567890"
            step="1"
            type="number"
            value={threadNumber}
          />
        </div>
        <button
          className="rounded-lg bg-blue-700 px-5 py-2.5 text-sm font-medium text-white hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
          disabled={resolveArchivedThread.isPending}
          type="submit"
        >
          {resolveArchivedThread.isPending ? "Searching..." : "Search"}
        </button>
      </form>
      {(inputError || resolveArchivedThread.isError) && (
        <p className="mt-3 text-sm text-red-600 dark:text-red-400">
          {inputError ?? "Archived thread was not found."}
        </p>
      )}
    </div>
  );
};

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

  return (
    <div className="mx-auto w-full max-w-7xl p-4 sm:p-6 lg:p-8">
      <h1 className="break-words text-2xl font-bold text-gray-900 sm:text-3xl">
        Threads: {params.boardKey}
      </h1>
      <Breadcrumb>
        <Link to="/dashboard/boards" className="text-gray-500 hover:text-gray-700">
          Boards
        </Link>
        <span className="text-gray-500" aria-current="page">
          Threads: {board?.name} ({params.boardKey})
        </span>
      </Breadcrumb>
      <Tab
        tabItems={[
          {
            tabKey: "threads",
            tabLabel: "Threads",
            id: "threads-tab",
            children: (
              <div className="p-2 sm:p-4">
                <ThreadsTabContent
                  boardKey={params.boardKey}
                  threads={
                    threads?.map((x) => ({
                      threadNumber: Number(x.thread_number),
                      title: x.title,
                      responseCount: Number(x.response_count),
                      archived: x.archived,
                    })) ?? []
                  }
                />
              </div>
            ),
          },
          {
            tabKey: "archived-threads",
            tabLabel: "Archives",
            id: "archived-threads-tab",
            children: (
              <div className="p-2 sm:p-4">
                <ArchivedThreadsTabContent boardKey={params.boardKey} />
              </div>
            ),
          },
          {
            tabKey: "settings",
            tabLabel: "Settings",
            id: "settings-tab",
            children: board ? <BoardSetting board={board} refetchBoard={refetch} /> : null,
          },
        ]}
      />
    </div>
  );
};

export default Page;
