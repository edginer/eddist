import { Button, Checkbox, HelperText, Label, Textarea, TextInput } from "flowbite-react";
import { useEffect, useMemo, useState } from "react";
import { getServerSettings, useUpsertServerSetting } from "~/hooks/queries";

type SettingDefinition =
  | {
      key: string;
      label: string;
      description: string;
      type: "boolean";
    }
  | {
      key: string;
      label: string;
      description: string;
      type: "text";
      placeholder?: string;
      sensitive?: boolean;
    }
  | {
      key: string;
      label: string;
      description: string;
      type: "textarea";
      placeholder?: string;
    };

const KNOWN_SETTINGS: SettingDefinition[] = [
  {
    key: "user.enable_idp_linking",
    label: "Enable IdP Linking",
    description: "Enable the IdP account linking feature.",
    type: "boolean",
  },
  {
    key: "user.require_idp_linking",
    label: "Require IdP Linking",
    description:
      "Require users to link an external IdP account before posting. Only applies to auth tokens issued after enabling this setting.",
    type: "boolean",
  },
  {
    key: "ai.openai_api_key",
    label: "OpenAI API Key",
    description:
      "API key used for OpenAI content moderation. Stored encrypted. Leave blank to keep existing value.",
    type: "text",
    placeholder: "sk-...",
    sensitive: true,
  },
  {
    key: "ai.moderation_on_res",
    label: "Enable Moderation for Responses",
    description:
      "Run OpenAI content moderation on each response before publishing the creation event.",
    type: "boolean",
  },
  {
    key: "ai.moderation_on_thread",
    label: "Enable Moderation for Threads",
    description:
      "Run OpenAI content moderation on each new thread (title + body) before publishing the creation event.",
    type: "boolean",
  },
  {
    key: "bbs.enable_safe_mode",
    label: "Enable Safe Mode",
    description:
      "Enable safe mode thread filtering. Hides threads with unsafe content from clients that support it.",
    type: "boolean",
  },
  {
    key: "ai.llm_moderation_on_thread",
    label: "Enable LLM Moderation for Threads",
    description:
      "Check new threads (title + body) with an LLM in batches in eddist-persistence and publish verdicts. Uses the OpenAI API key above.",
    type: "boolean",
  },
  {
    key: "ai.llm_moderation_unsafe_threads",
    label: "Use LLM Verdicts for Safe Mode",
    description:
      'Decide which threads safe mode hides from LLM verdicts instead of the OpenAI moderation API. Requires LLM moderation to be enabled; turn off "Enable Moderation for Threads" once this is on.',
    type: "boolean",
  },
  {
    key: "ai.llm_moderation_model",
    label: "LLM Moderation Model",
    description: "Model for LLM thread moderation. Leave blank for gpt-5.6-luna.",
    type: "text",
    placeholder: "gpt-5.6-luna",
  },
  {
    key: "ai.llm_moderation_interval_seconds",
    label: "LLM Moderation Interval (seconds)",
    description:
      "How many seconds of new threads are batched into one LLM request. Shorter means faster safe-mode updates but more tokens per day, because the instructions are sent with every request. Leave blank for 120; clamped to 10-3600.",
    type: "text",
    placeholder: "120",
  },
  {
    key: "ai.llm_moderation_instructions",
    label: "LLM Moderation Instructions",
    description:
      "Moderation policy given to the LLM, written against your terms of service. Required: while empty, batches are dropped. The output format is appended automatically; clause keys must be defamation, ip_infringement, personal_info, illegal, violent_or_csam, misinformation, spam or discrimination.",
    type: "textarea",
  },
];

const MASKED_VALUE = "***";

const isSensitive = (def: SettingDefinition) => def.type === "text" && def.sensitive === true;

const ServerSettings = () => {
  const { data: settings } = getServerSettings();
  const upsertMutation = useUpsertServerSetting();

  const settingMap = useMemo(
    () => new Map(settings?.map((s) => [s.setting_key, s]) ?? []),
    [settings],
  );

  const [values, setValues] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const initial: Record<string, string> = {};
    for (const def of KNOWN_SETTINGS) {
      const existing = settingMap.get(def.key);
      if (def.type === "boolean") {
        initial[def.key] = existing?.value ?? "false";
      } else if (!isSensitive(def)) {
        initial[def.key] = existing?.value ?? "";
      } else {
        // Server returns "***" for sensitive fields already set; start empty so the user types a new value.
        initial[def.key] = "";
      }
    }
    setValues(initial);
  }, [settingMap]);

  const handleToggle = (key: string, checked: boolean) => {
    setValues((prev) => ({ ...prev, [key]: checked ? "true" : "false" }));
  };

  const handleText = (key: string, value: string) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      for (const def of KNOWN_SETTINGS) {
        const existing = settingMap.get(def.key);
        const newValue = values[def.key] ?? "";

        if (def.type === "boolean") {
          const savedValue = existing?.value ?? "false";
          if (newValue !== savedValue) {
            await upsertMutation.mutateAsync({
              body: {
                setting_key: def.key,
                value: newValue,
                description: def.description,
              },
            });
          }
        } else if (!isSensitive(def)) {
          if (newValue !== (existing?.value ?? "")) {
            await upsertMutation.mutateAsync({
              body: {
                setting_key: def.key,
                value: newValue,
                description: def.description,
              },
            });
          }
        } else {
          // Empty = no change; skip "***" to avoid re-submitting the masked sentinel.
          if (newValue.length > 0 && newValue !== MASKED_VALUE) {
            await upsertMutation.mutateAsync({
              body: {
                setting_key: def.key,
                value: newValue,
                description: def.description,
              },
            });
          }
        }
      }
    } finally {
      setSaving(false);
    }
  };

  const hasChanges = KNOWN_SETTINGS.some((def) => {
    const currentValue = values[def.key] ?? "";
    if (def.type === "boolean") {
      const savedValue = settingMap.get(def.key)?.value ?? "false";
      return currentValue !== savedValue;
    }
    if (!isSensitive(def)) {
      return currentValue !== (settingMap.get(def.key)?.value ?? "");
    }
    return currentValue.length > 0 && currentValue !== MASKED_VALUE;
  });

  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold mb-6">Server Settings</h1>

      <div className="flex flex-col gap-6 max-w-2xl">
        {KNOWN_SETTINGS.map((def) => (
          <div key={def.key} className="flex flex-col gap-1">
            {def.type === "boolean" ? (
              <>
                <div className="flex items-center gap-3">
                  <Checkbox
                    id={def.key}
                    checked={values[def.key] === "true"}
                    onChange={(e) => handleToggle(def.key, e.target.checked)}
                  />
                  <Label htmlFor={def.key} className="text-base">
                    {def.label}
                  </Label>
                </div>
                <HelperText className="ml-7">{def.description}</HelperText>
              </>
            ) : def.type === "textarea" ? (
              <>
                <Label htmlFor={def.key} className="text-base">
                  {def.label}
                </Label>
                <Textarea
                  id={def.key}
                  rows={16}
                  placeholder={def.placeholder}
                  value={values[def.key] ?? ""}
                  onChange={(e) => handleText(def.key, e.target.value)}
                  className="font-mono text-sm"
                />
                <HelperText>{def.description}</HelperText>
              </>
            ) : (
              <>
                <Label htmlFor={def.key} className="text-base">
                  {def.label}
                  {settingMap.get(def.key)?.value === MASKED_VALUE && (
                    <span className="ml-2 text-xs text-gray-500 font-normal">(already set)</span>
                  )}
                </Label>
                <TextInput
                  id={def.key}
                  type={def.sensitive ? "password" : "text"}
                  placeholder={def.placeholder}
                  value={values[def.key] ?? ""}
                  onChange={(e) => handleText(def.key, e.target.value)}
                  autoComplete="off"
                />
                <HelperText>{def.description}</HelperText>
              </>
            )}
          </div>
        ))}

        <div className="mt-2">
          <Button onClick={handleSave} disabled={saving || !hasChanges}>
            {saving ? "Saving..." : "Save"}
          </Button>
        </div>
      </div>
    </div>
  );
};

export default ServerSettings;
