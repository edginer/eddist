import {
  Button,
  Modal,
  ModalBody,
  ModalFooter,
  ModalHeader,
  Tooltip,
} from "flowbite-react";
import { HiInformationCircle } from "react-icons/hi";
import { useNGWords } from "~/contexts/NGWordsContext";
import { Tabs } from "./Tabs";
import { NGRuleSection } from "./NGRuleSection";

interface NGWordsSettingsModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

export const NGWordsSettingsModal = ({
  open,
  setOpen,
}: NGWordsSettingsModalProps) => {
  const { config, addRule, updateRule, removeRule, toggleRule, clearAllRules } =
    useNGWords();

  return (
    <Modal show={open} size="5xl" onClose={() => setOpen(false)}>
      <ModalHeader className="border-gray-200">
        <div className="flex items-center gap-2">
          <span className="lg:text-2xl">NG設定</span>
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
              title: "スレッド",
              content: (
                <>
                  <NGRuleSection
                    title="投稿者ID"
                    rules={config.thread.authorIds}
                    onAdd={(rule) => addRule("thread.authorIds", rule)}
                    onUpdate={(id, updates) =>
                      updateRule("thread.authorIds", id, updates)
                    }
                    onRemove={(id) => removeRule("thread.authorIds", id)}
                    onToggle={(id) => toggleRule("thread.authorIds", id)}
                    isResponseRule={false}
                  />
                  <NGRuleSection
                    title="スレッドタイトル"
                    rules={config.thread.titles}
                    onAdd={(rule) => addRule("thread.titles", rule)}
                    onUpdate={(id, updates) =>
                      updateRule("thread.titles", id, updates)
                    }
                    onRemove={(id) => removeRule("thread.titles", id)}
                    onToggle={(id) => toggleRule("thread.titles", id)}
                    isResponseRule={false}
                  />
                </>
              ),
            },
            {
              id: "response",
              title: "レス",
              content: (
                <>
                  <NGRuleSection
                    title="投稿者ID"
                    rules={config.response.authorIds}
                    onAdd={(rule) => addRule("response.authorIds", rule)}
                    onUpdate={(id, updates) =>
                      updateRule("response.authorIds", id, updates)
                    }
                    onRemove={(id) => removeRule("response.authorIds", id)}
                    onToggle={(id) => toggleRule("response.authorIds", id)}
                    isResponseRule={true}
                  />
                  <NGRuleSection
                    title="本文"
                    rules={config.response.bodies}
                    onAdd={(rule) => addRule("response.bodies", rule)}
                    onUpdate={(id, updates) =>
                      updateRule("response.bodies", id, updates)
                    }
                    onRemove={(id) => removeRule("response.bodies", id)}
                    onToggle={(id) => toggleRule("response.bodies", id)}
                    isResponseRule={true}
                  />
                  <NGRuleSection
                    title="投稿者名"
                    rules={config.response.names}
                    onAdd={(rule) => addRule("response.names", rule)}
                    onUpdate={(id, updates) =>
                      updateRule("response.names", id, updates)
                    }
                    onRemove={(id) => removeRule("response.names", id)}
                    onToggle={(id) => toggleRule("response.names", id)}
                    isResponseRule={true}
                  />
                </>
              ),
            },
          ]}
        />
      </ModalBody>
      <ModalFooter className="flex justify-between mt-6  border-t border-gray-200">
        <Button
          color="gray"
          onClick={() => {
            if (
              window.confirm(
                "すべてのNG設定をクリアしますか？\nこの操作は取り消せません。"
              )
            ) {
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
