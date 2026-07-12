import { Button, Modal, ModalBody, ModalFooter, ModalHeader, Tooltip } from "flowbite-react";
import { useState } from "react";
import { FaDesktop, FaMoon, FaSun } from "react-icons/fa";
import { HiInformationCircle } from "react-icons/hi";
import { useRevalidator } from "react-router";
import { deleteSharedNgId } from "~/api-client/ng_id";
import { useNGWords } from "~/contexts/NGWordsContext";
import { useTheme } from "~/contexts/ThemeContext";
import { parseCookie } from "~/utils/cookie";
import { NGRuleSection } from "./NGRuleSection";
import { Tabs } from "./Tabs";

interface NGWordsSettingsModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  enableSafeMode: boolean;
  summarizerSupported?: boolean;
  summarizeEnabled?: boolean;
  onSummarizeEnabledChange?: (enabled: boolean) => void;
}

const ThemeTab = () => {
  const { theme, setTheme } = useTheme();

  const options = [
    { value: "system", label: "システムデフォルト", icon: FaDesktop },
    { value: "light", label: "ライト", icon: FaSun },
    { value: "dark", label: "ダーク", icon: FaMoon },
  ] as const;

  return (
    <div className="py-2 dark:text-gray-100">
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
  );
};

const SafeModeTab = () => {
  const { revalidate } = useRevalidator();
  const [safeMode, setSafeMode] = useState(() => {
    if (typeof document === "undefined") return true;
    return parseCookie(document.cookie, "safe_mode") !== "off";
  });

  const handleToggle = () => {
    const next = !safeMode;
    if (next) {
      document.cookie = "safe_mode=; path=/; max-age=0; SameSite=Lax";
    } else {
      document.cookie = "safe_mode=off; path=/; max-age=31536000; SameSite=Lax";
    }
    setSafeMode(next);
    revalidate();
  };

  return (
    <div className="py-2 dark:text-gray-100">
      <h3 className="text-lg font-semibold mb-2">セーフモード</h3>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
        ONのとき、不適切と判定されたスレッドをスレッド一覧から非表示にします。スレッドURLへの直接アクセスは引き続き可能です。
      </p>
      <label className="flex items-center gap-3 cursor-pointer select-none">
        <div className="relative">
          <input type="checkbox" className="sr-only" checked={safeMode} onChange={handleToggle} />
          <div
            className={`w-11 h-6 rounded-full transition-colors ${
              safeMode ? "bg-blue-600" : "bg-gray-300 dark:bg-gray-600"
            }`}
          />
          <div
            className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
              safeMode ? "translate-x-5" : ""
            }`}
          />
        </div>
        <span className="font-medium">セーフモード: {safeMode ? "ON" : "OFF"}</span>
      </label>
    </div>
  );
};

const SummarizeTab = ({
  enabled,
  onChange,
}: {
  enabled: boolean;
  onChange: (v: boolean) => void;
}) => (
  <div className="py-2 dark:text-gray-100">
    <h3 className="text-lg font-semibold mb-2">スレッド要約</h3>
    <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
      ONのとき、スレッド一覧にAI要約ボタンを表示します。ChromiumベースのブラウザのSummarizer
      APIを使用します。初回使用時にAIモデルの大容量データ（数GB）がダウンロードされる場合があります。モバイル回線などでのご利用はご注意ください。内容の正確性は保証しません。
    </p>
    <label className="flex items-center gap-3 cursor-pointer select-none">
      <div className="relative">
        <input
          type="checkbox"
          className="sr-only"
          checked={enabled}
          onChange={() => onChange(!enabled)}
        />
        <div
          className={`w-11 h-6 rounded-full transition-colors ${
            enabled ? "bg-blue-600" : "bg-gray-300 dark:bg-gray-600"
          }`}
        />
        <div
          className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
            enabled ? "translate-x-5" : ""
          }`}
        />
      </div>
      <span className="font-medium">スレッド要約: {enabled ? "ON" : "OFF"}</span>
    </label>
  </div>
);

export const NGWordsSettingsModal = ({
  open,
  setOpen,
  enableSafeMode,
  summarizerSupported,
  summarizeEnabled = false,
  onSummarizeEnabledChange,
}: NGWordsSettingsModalProps) => {
  const { config, addRule, updateRule, removeRule, toggleRule, clearAllRules } = useNGWords();

  // Removing a synced response 投稿者ID rule also retracts its shared NG ID.
  const removeResponseAuthorId = (ruleId: string) => {
    const rule = config.response.authorIds.find((r) => r.id === ruleId);
    if (rule?.sharedBoardKey) {
      void deleteSharedNgId(rule.sharedBoardKey, rule.pattern);
    }
    removeRule("response.authorIds", ruleId);
  };

  const handleClearAll = () => {
    if (!window.confirm("すべてのNG設定をクリアしますか？\nこの操作は取り消せません。")) {
      return;
    }
    // Retract synced shared NG IDs before wiping local config.
    for (const rule of config.response.authorIds) {
      if (rule.sharedBoardKey) {
        void deleteSharedNgId(rule.sharedBoardKey, rule.pattern);
      }
    }
    clearAllRules();
  };

  return (
    <Modal show={open} size="5xl" onClose={() => setOpen(false)} dismissible>
      <ModalHeader className="border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <span className="lg:text-2xl">設定</span>
          <Tooltip content="この設定は端末内（ローカルストレージ）に保存されます。ただしレス一覧から追加したNG IDはサーバーにも共有されます。">
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
                    onRemove={removeResponseAuthorId}
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
              title: "テーマ",
              content: <ThemeTab />,
            },
            ...(enableSafeMode
              ? [{ id: "safe-mode", title: "セーフモード", content: <SafeModeTab /> }]
              : []),
            ...(summarizerSupported
              ? [
                  {
                    id: "summarize",
                    title: "要約",
                    content: (
                      <SummarizeTab
                        enabled={summarizeEnabled}
                        onChange={onSummarizeEnabledChange ?? (() => {})}
                      />
                    ),
                  },
                ]
              : []),
          ]}
        />
      </ModalBody>
      <ModalFooter className="flex justify-between mt-6 border-t border-gray-200 dark:border-gray-700">
        <Button color="gray" onClick={handleClearAll}>
          すべてクリア
        </Button>
        <Button onClick={() => setOpen(false)}>閉じる</Button>
      </ModalFooter>
    </Modal>
  );
};
