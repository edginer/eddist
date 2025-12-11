import { Button, TextInput } from "flowbite-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { HiPencil } from "react-icons/hi";
import type { NGRule } from "~/contexts/NGWordsContext";

interface NGRuleFormData {
  pattern: string;
  matchType: "partial" | "regex";
  hideMode: "hidden" | "collapsed";
}

interface NGRuleSectionProps {
  title: string;
  rules: NGRule[];
  onAdd: (rule: Omit<NGRule, "id">) => void;
  onUpdate: (ruleId: string, updates: Partial<Omit<NGRule, "id">>) => void;
  onRemove: (ruleId: string) => void;
  onToggle: (ruleId: string) => void;
  placeholder?: string;
  isResponseRule?: boolean;
}

export const NGRuleSection = ({
  title,
  rules,
  onAdd,
  onUpdate,
  onRemove,
  onToggle,
  placeholder,
  isResponseRule = false,
}: NGRuleSectionProps) => {
  const [editingId, setEditingId] = useState<string | null>(null);

  const addForm = useForm<NGRuleFormData>({
    defaultValues: {
      pattern: "",
      matchType: "partial",
      hideMode: "collapsed",
    },
  });

  const editForm = useForm<NGRuleFormData>({
    defaultValues: {
      pattern: "",
      matchType: "partial",
      hideMode: "collapsed",
    },
  });

  const {
    register: addRegister,
    handleSubmit: addHandleSubmit,
    watch: addWatch,
    reset: addReset,
  } = addForm;

  const {
    register: editRegister,
    handleSubmit: editHandleSubmit,
    watch: editWatch,
    reset: editReset,
  } = editForm;

  // Watch for radio button state
  const newMatchType = addWatch("matchType");
  const newHideMode = addWatch("hideMode");
  const editMatchType = editWatch("matchType");
  const editHideMode = editWatch("hideMode");

  const handleAdd = addHandleSubmit((data) => {
    onAdd({
      pattern: data.pattern.trim(),
      matchType: data.matchType,
      enabled: true,
      ...(isResponseRule ? { hideMode: data.hideMode } : {}),
    });

    addReset();
  });

  const handleStartEdit = (rule: NGRule) => {
    setEditingId(rule.id);
    editReset({
      pattern: rule.pattern,
      matchType: rule.matchType,
      hideMode: rule.hideMode || "collapsed",
    });
  };

  const handleSaveEdit = editHandleSubmit((data) => {
    if (!editingId) return;

    onUpdate(editingId, {
      pattern: data.pattern.trim(),
      matchType: data.matchType,
      ...(isResponseRule ? { hideMode: data.hideMode } : {}),
    });

    setEditingId(null);
    editReset();
  });

  const handleCancelEdit = () => {
    setEditingId(null);
    editReset();
  };

  return (
    <div className="mb-6 border-b pb-4 last:border-b-0">
      <h3 className="text-lg font-semibold mb-3">{title}</h3>

      {/* Add new rule */}
      <form onSubmit={handleAdd} className="mb-4">
        <div className="flex flex-col gap-3">
          <TextInput
            placeholder={placeholder}
            {...addRegister("pattern", { required: true })}
            className="flex-1"
          />

          <div className="flex flex-wrap gap-4 items-center">
            {/* Match type */}
            <div className="flex gap-3">
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="radio"
                  {...addRegister("matchType")}
                  value="partial"
                  checked={newMatchType === "partial"}
                  className="cursor-pointer"
                />
                <span className="text-sm">部分一致</span>
              </label>
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="radio"
                  {...addRegister("matchType")}
                  value="regex"
                  checked={newMatchType === "regex"}
                  className="cursor-pointer"
                />
                <span className="text-sm">正規表現</span>
              </label>
            </div>

            {/* Hide mode (only for response rules) */}
            {isResponseRule && (
              <div className="flex gap-3 pl-4 border-l border-gray-300">
                <label className="flex items-center gap-1.5 cursor-pointer">
                  <input
                    type="radio"
                    {...addRegister("hideMode")}
                    value="collapsed"
                    checked={newHideMode === "collapsed"}
                    className="cursor-pointer"
                  />
                  <span className="text-sm">折りたたむ</span>
                </label>
                <label className="flex items-center gap-1.5 cursor-pointer">
                  <input
                    type="radio"
                    {...addRegister("hideMode")}
                    value="hidden"
                    checked={newHideMode === "hidden"}
                    className="cursor-pointer"
                  />
                  <span className="text-sm">完全非表示</span>
                </label>
              </div>
            )}

            <Button type="submit" size="sm" className="ml-auto">
              追加
            </Button>
          </div>
        </div>
      </form>

      {/* Existing rules list */}
      {rules.length === 0 ? (
        <p className="text-gray-400 text-sm italic">NG設定がありません</p>
      ) : (
        <ul className="space-y-2">
          {rules.map((rule) => (
            <li
              key={rule.id}
              className="bg-gray-50 p-3 rounded hover:bg-gray-100 transition-colors"
            >
              {editingId === rule.id ? (
                /* Edit mode */
                <div className="space-y-3">
                  <TextInput
                    {...editRegister("pattern", { required: true })}
                    className="w-full"
                  />
                  <div className="flex flex-wrap gap-4 items-center">
                    {/* Match type */}
                    <div className="flex gap-3">
                      <label className="flex items-center gap-1.5 cursor-pointer">
                        <input
                          type="radio"
                          {...editRegister("matchType")}
                          value="partial"
                          checked={editMatchType === "partial"}
                          className="cursor-pointer"
                        />
                        <span className="text-sm">部分一致</span>
                      </label>
                      <label className="flex items-center gap-1.5 cursor-pointer">
                        <input
                          type="radio"
                          {...editRegister("matchType")}
                          value="regex"
                          checked={editMatchType === "regex"}
                          className="cursor-pointer"
                        />
                        <span className="text-sm">正規表現</span>
                      </label>
                    </div>

                    {/* Hide mode (only for response rules) */}
                    {isResponseRule && (
                      <div className="flex gap-3 pl-4 border-l border-gray-300">
                        <label className="flex items-center gap-1.5 cursor-pointer">
                          <input
                            type="radio"
                            {...editRegister("hideMode")}
                            value="collapsed"
                            checked={editHideMode === "collapsed"}
                            className="cursor-pointer"
                          />
                          <span className="text-sm">折りたたむ</span>
                        </label>
                        <label className="flex items-center gap-1.5 cursor-pointer">
                          <input
                            type="radio"
                            {...editRegister("hideMode")}
                            value="hidden"
                            checked={editHideMode === "hidden"}
                            className="cursor-pointer"
                          />
                          <span className="text-sm">完全非表示</span>
                        </label>
                      </div>
                    )}

                    <div className="ml-auto flex gap-2">
                      <Button size="xs" onClick={handleSaveEdit}>
                        保存
                      </Button>
                      <Button size="xs" color="gray" onClick={handleCancelEdit}>
                        キャンセル
                      </Button>
                    </div>
                  </div>
                </div>
              ) : (
                /* View mode */
                <div className="flex justify-between items-center">
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    <input
                      type="checkbox"
                      checked={rule.enabled}
                      onChange={() => onToggle(rule.id)}
                      className="cursor-pointer shrink-0"
                      title={rule.enabled ? "無効化" : "有効化"}
                    />
                    <div className="flex-1 min-w-0">
                      <span
                        className={
                          !rule.enabled
                            ? "text-gray-400 line-through break-all"
                            : "break-all"
                        }
                      >
                        {rule.pattern}
                      </span>
                      <div className="text-xs text-gray-500 mt-1">
                        (
                        {rule.matchType === "partial" ? "部分一致" : "正規表現"}
                        )
                        {rule.hideMode &&
                          ` - ${
                            rule.hideMode === "collapsed"
                              ? "折りたたむ"
                              : "完全非表示"
                          }`}
                      </div>
                    </div>
                  </div>
                  <div className="flex gap-1 ml-2 shrink-0">
                    <button
                      onClick={() => handleStartEdit(rule)}
                      className="text-blue-500 hover:text-blue-700 p-2 rounded hover:bg-blue-50 transition-colors"
                      type="button"
                      title="編集"
                    >
                      <HiPencil className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => onRemove(rule.id)}
                      className="text-red-500 hover:text-red-700 text-sm px-2 py-1 rounded hover:bg-red-50 transition-colors"
                      type="button"
                    >
                      削除
                    </button>
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
