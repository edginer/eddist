import { Modal, ModalBody, ModalHeader } from "flowbite-react";
import { FaTrash } from "react-icons/fa";
import { Link } from "react-router";
import {
  type FavoriteEntry,
  type PostHistoryEntry,
  type ReadHistoryEntry,
  useThreadHistory,
} from "~/contexts/ThreadHistoryContext";
import { useUISettings } from "~/contexts/UISettingsContext";
import { Tabs } from "./Tabs";

const formatDate = (ms: number): string => {
  const date = new Date(ms);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hour = String(date.getHours()).padStart(2, "0");
  const minute = String(date.getMinutes()).padStart(2, "0");
  return `${year}/${month}/${day} ${hour}:${minute}`;
};

const HistoryList = ({ onClose }: { onClose: () => void }) => {
  const { history, removeHistoryEntry, clearHistory } = useThreadHistory();

  if (history.length === 0) {
    return <p className="text-gray-500 dark:text-gray-400 text-sm py-4">閲覧履歴はありません。</p>;
  }

  return (
    <div>
      <div className="flex justify-end mb-2">
        <button
          type="button"
          onClick={clearHistory}
          className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
        >
          すべて削除
        </button>
      </div>
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {history.map((entry: ReadHistoryEntry) => (
          <li key={entry.key} className="flex items-start justify-between py-3 gap-2">
            <Link
              to={`/${entry.boardKey}/${entry.threadKey}`}
              className="flex-1 min-w-0 hover:underline"
              onClick={onClose}
            >
              <p className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                {entry.title}
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                {entry.boardKey} &middot; {entry.lastReadCount}レス &middot;{" "}
                {formatDate(entry.visitedAt)}
              </p>
            </Link>
            <button
              type="button"
              aria-label="削除"
              onClick={() => removeHistoryEntry(entry.key)}
              className="shrink-0 text-gray-400 hover:text-red-500 dark:hover:text-red-400 mt-0.5"
            >
              <FaTrash size={12} />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
};

const FavoritesList = ({ onClose }: { onClose: () => void }) => {
  const { favorites, removeFavorite } = useThreadHistory();

  if (favorites.length === 0) {
    return (
      <p className="text-gray-500 dark:text-gray-400 text-sm py-4">
        お気に入りに登録されたスレッドはありません。
      </p>
    );
  }

  return (
    <ul className="divide-y divide-gray-200 dark:divide-gray-700">
      {favorites.map((entry: FavoriteEntry) => (
        <li key={entry.key} className="flex items-start justify-between py-3 gap-2">
          <Link
            to={`/${entry.boardKey}/${entry.threadKey}`}
            className="flex-1 min-w-0 hover:underline"
            onClick={onClose}
          >
            <p className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
              {entry.title}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              {entry.boardKey} &middot; {entry.lastReadCount}レス &middot;{" "}
              {formatDate(entry.favoritedAt)}
            </p>
          </Link>
          <button
            type="button"
            aria-label="お気に入り解除"
            onClick={() => removeFavorite(entry.key)}
            className="shrink-0 text-gray-400 hover:text-red-500 dark:hover:text-red-400 mt-0.5"
          >
            <FaTrash size={12} />
          </button>
        </li>
      ))}
    </ul>
  );
};

const PostHistoryList = ({ onClose }: { onClose: () => void }) => {
  const { postHistory, removePostHistoryEntry, clearPostHistory } = useThreadHistory();

  if (postHistory.length === 0) {
    return <p className="text-gray-500 dark:text-gray-400 text-sm py-4">投稿履歴はありません。</p>;
  }

  return (
    <div>
      <div className="flex justify-end mb-2">
        <button
          type="button"
          onClick={clearPostHistory}
          className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
        >
          すべて削除
        </button>
      </div>
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {postHistory.map((entry: PostHistoryEntry) => (
          <li key={entry.key} className="flex items-start justify-between py-3 gap-2">
            <Link
              to={`/${entry.boardKey}/${entry.threadKey}`}
              className="flex-1 min-w-0 hover:underline"
              onClick={onClose}
            >
              <p className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                {entry.threadTitle}
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                {entry.boardKey}
                {entry.name && <> &middot; {entry.name}</>} &middot; {formatDate(entry.postedAt)}
              </p>
              <p className="text-xs text-gray-600 dark:text-gray-300 mt-1 line-clamp-2">
                {entry.body.length > 100 ? `${entry.body.slice(0, 100)}…` : entry.body}
              </p>
            </Link>
            <button
              type="button"
              aria-label="削除"
              onClick={() => removePostHistoryEntry(entry.key)}
              className="shrink-0 text-gray-400 hover:text-red-500 dark:hover:text-red-400 mt-0.5"
            >
              <FaTrash size={12} />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
};

interface HistoryModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

export const HistoryModal = ({ open, setOpen }: HistoryModalProps) => {
  const onClose = () => setOpen(false);
  const { settings } = useUISettings();

  const tabs = [
    settings.enableFavorites && {
      id: "favorites",
      title: "★ お気に入り",
      content: <FavoritesList onClose={onClose} />,
    },
    settings.enableReadHistory && {
      id: "history",
      title: "閲覧履歴",
      content: <HistoryList onClose={onClose} />,
    },
    settings.enablePostHistory && {
      id: "post_history",
      title: "投稿履歴",
      content: <PostHistoryList onClose={onClose} />,
    },
  ].filter(Boolean) as { id: string; title: string; content: React.ReactNode }[];

  return (
    <Modal show={open} size="2xl" onClose={onClose} dismissible>
      <ModalHeader>履歴・お気に入り</ModalHeader>
      <ModalBody>
        {tabs.length > 0 ? (
          <Tabs tabs={tabs} defaultTab={tabs[0].id} />
        ) : (
          <p className="text-gray-500 dark:text-gray-400 text-sm py-4">
            履歴・お気に入り機能はすべて無効になっています。
          </p>
        )}
      </ModalBody>
    </Modal>
  );
};
