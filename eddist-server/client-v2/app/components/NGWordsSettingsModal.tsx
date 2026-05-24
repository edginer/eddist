import { Button, Modal, ModalBody, ModalFooter, ModalHeader, Tooltip } from "flowbite-react";
import { useState } from "react";
import { FaDesktop, FaMoon, FaSun } from "react-icons/fa";
import { HiInformationCircle } from "react-icons/hi";
import { useNGWords } from "~/contexts/NGWordsContext";
import { useTheme } from "~/contexts/ThemeContext";
import { useThreadHistory } from "~/contexts/ThreadHistoryContext";
import { useUISettings } from "~/contexts/UISettingsContext";
import { NGRuleSection } from "./NGRuleSection";
import { Tabs } from "./Tabs";

interface NGWordsSettingsModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

type ConfirmTarget = "history" | "favorites" | "post_history";

const CONFIRM_TEXT: Record<ConfirmTarget, { title: string; body: string }> = {
  history: {
    title: "閲覧履歴の削除",
    body: "閲覧履歴をすべて削除して機能を無効にします。この操作は取り消せません。",
  },
  favorites: {
    title: "お気に入りの削除",
    body: "お気に入りをすべて削除して機能を無効にします。この操作は取り消せません。",
  },
  post_history: {
    title: "投稿履歴の削除",
    body: "投稿履歴をすべて削除して機能を無効にします。この操作は取り消せません。",
  },
};

const ThemeTab = () => {
  const { theme, setTheme } = useTheme();
  const { settings, setSetting } = useUISettings();
  const { clearHistory, favorites, removeFavorite, clearPostHistory } = useThreadHistory();
  const [confirmTarget, setConfirmTarget] = useState<ConfirmTarget | null>(null);

  const handleConfirm = () => {
    if (confirmTarget === "history") {
      clearHistory();
      setSetting("enableReadHistory", false);
    } else if (confirmTarget === "favorites") {
      for (const fav of favorites) removeFavorite(fav.key);
      setSetting("enableFavorites", false);
    } else if (confirmTarget === "post_history") {
      clearPostHistory();
      setSetting("enablePostHistory", false);
    }
    setConfirmTarget(null);
  };

  const options = [
    { value: "system", label: "システムデフォルト", icon: FaDesktop },
    { value: "light", label: "ライト", icon: FaSun },
    { value: "dark", label: "ダーク", icon: FaMoon },
  ] as const;

  return (
    <>
      {confirmTarget && (
        <Modal show size="sm" onClose={() => setConfirmTarget(null)} dismissible>
          <ModalHeader>{CONFIRM_TEXT[confirmTarget].title}</ModalHeader>
          <ModalBody>
            <p className="text-gray-700 dark:text-gray-300">{CONFIRM_TEXT[confirmTarget].body}</p>
          </ModalBody>
          <ModalFooter className="flex justify-end gap-2">
            <Button color="gray" onClick={() => setConfirmTarget(null)}>
              キャンセル
            </Button>
            <Button color="red" onClick={handleConfirm}>
              削除して無効にする
            </Button>
          </ModalFooter>
        </Modal>
      )}
      <div className="py-2 dark:text-gray-100 flex flex-col gap-6">
        <div>
          <h3 className="text-lg font-semibold mb-4">テーマ</h3>
          <div className="flex flex-col gap-3">
            {options.map(({ value, label, icon: Icon }) => (
              <label key={value} className="flex items-center gap-3 cursor-pointer">
                <input
                  type="radio"
                  name="theme"
                  value={value}
                  checked={theme === value}
                  onChange={() => setTheme(value)}
                  className="cursor-pointer"
                />
                <Icon className="w-4 h-4 text-gray-500 dark:text-gray-400" />
                <span>{label}</span>
              </label>
            ))}
          </div>
        </div>
        <div>
          <h3 className="text-lg font-semibold mb-4">表示</h3>
          <div className="flex flex-col gap-3">
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.showHistoryButtons}
                onChange={(e) => setSetting("showHistoryButtons", e.target.checked)}
                className="cursor-pointer"
              />
              <span>ヘッダーに履歴・お気に入りボタンを表示する</span>
            </label>
          </div>
        </div>
        <div>
          <h3 className="text-lg font-semibold mb-4">履歴・お気に入り</h3>
          <div className="flex flex-col gap-3">
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.enableReadHistory}
                onChange={(e) => {
                  if (!e.target.checked) setConfirmTarget("history");
                  else setSetting("enableReadHistory", true);
                }}
                className="cursor-pointer"
              />
              <span>閲覧履歴を記録する</span>
            </label>
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.enableFavorites}
                onChange={(e) => {
                  if (!e.target.checked) setConfirmTarget("favorites");
                  else setSetting("enableFavorites", true);
                }}
                className="cursor-pointer"
              />
              <span>お気に入り機能を有効にする</span>
            </label>
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.enablePostHistory}
                onChange={(e) => {
                  if (!e.target.checked) setConfirmTarget("post_history");
                  else setSetting("enablePostHistory", true);
                }}
                className="cursor-pointer"
              />
              <span>投稿履歴を記録する</span>
            </label>
          </div>
        </div>
      </div>
    </>
  );
};

export const NGWordsSettingsModal = ({ open, setOpen }: NGWordsSettingsModalProps) => {
  const { config, addRule, updateRule, removeRule, toggleRule, clearAllRules } = useNGWords();

  return (
    <Modal show={open} size="5xl" onClose={() => setOpen(false)} dismissible>
      <ModalHeader className="border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <span className="lg:text-2xl">設定</span>
          <Tooltip content="この設定はローカルストレージに保存されます">
            <HiInformationCircle className="w-5 h-5 text-gray-400 hover:text-gray-600 cursor-help" />
          </Tooltip>
        </div>
      </ModalHeader>
      <ModalBody className="pb-0">
        <Tabs
          tabs={[
            {
              id: "thread",
              title: "スレッドNG",
              content: (
                <>
                  <NGRuleSection
                    title="投稿者ID"
                    rules={config.thread.authorIds}
                    onAdd={(rule) => addRule("thread.authorIds", rule)}
                    onUpdate={(id, updates) => updateRule("thread.authorIds", id, updates)}
                    onRemove={(id) => removeRule("thread.authorIds", id)}
                    onToggle={(id) => toggleRule("thread.authorIds", id)}
                    isResponseRule={false}
                  />
                  <NGRuleSection
                    title="スレッドタイトル"
                    rules={config.thread.titles}
                    onAdd={(rule) => addRule("thread.titles", rule)}
                    onUpdate={(id, updates) => updateRule("thread.titles", id, updates)}
                    onRemove={(id) => removeRule("thread.titles", id)}
                    onToggle={(id) => toggleRule("thread.titles", id)}
                    isResponseRule={false}
                  />
                </>
              ),
            },
            {
              id: "response",
              title: "レスNG",
              content: (
                <>
                  <NGRuleSection
                    title="投稿者ID"
                    rules={config.response.authorIds}
                    onAdd={(rule) => addRule("response.authorIds", rule)}
                    onUpdate={(id, updates) => updateRule("response.authorIds", id, updates)}
                    onRemove={(id) => removeRule("response.authorIds", id)}
                    onToggle={(id) => toggleRule("response.authorIds", id)}
                    isResponseRule={true}
                  />
                  <NGRuleSection
                    title="本文"
                    rules={config.response.bodies}
                    onAdd={(rule) => addRule("response.bodies", rule)}
                    onUpdate={(id, updates) => updateRule("response.bodies", id, updates)}
                    onRemove={(id) => removeRule("response.bodies", id)}
                    onToggle={(id) => toggleRule("response.bodies", id)}
                    isResponseRule={true}
                  />
                  <NGRuleSection
                    title="投稿者名"
                    rules={config.response.names}
                    onAdd={(rule) => addRule("response.names", rule)}
                    onUpdate={(id, updates) => updateRule("response.names", id, updates)}
                    onRemove={(id) => removeRule("response.names", id)}
                    onToggle={(id) => toggleRule("response.names", id)}
                    isResponseRule={true}
                  />
                </>
              ),
            },
            {
              id: "theme",
              title: "テーマ・表示",
              content: <ThemeTab />,
            },
          ]}
        />
      </ModalBody>
      <ModalFooter className="flex justify-between mt-6 border-t border-gray-200 dark:border-gray-700">
        <Button
          color="gray"
          onClick={() => {
            if (window.confirm("すべてのNG設定をクリアしますか？\nこの操作は取り消せません。")) {
              clearAllRules();
            }
          }}
        >
          すべてクリア
        </Button>
        <Button onClick={() => setOpen(false)}>閉じる</Button>
      </ModalFooter>
    </Modal>
  );
};
