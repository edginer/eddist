import { Button, Checkbox, HelperText, Label } from "flowbite-react";
import { useEffect, useState } from "react";
import { getServerSettings, useUpsertServerSetting } from "~/hooks/queries";

type SettingDefinition = {
  key: string;
  label: string;
  description: string;
  type: "boolean";
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
      "Require users to link an external IdP account before posting.",
    type: "boolean",
  },
];

const ServerSettings = () => {
  const { data: settings } = getServerSettings();
  const upsertMutation = useUpsertServerSetting();

  const [values, setValues] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (settings) {
      const initial: Record<string, string> = {};
      for (const def of KNOWN_SETTINGS) {
        const existing = settings.find((s) => s.setting_key === def.key);
        initial[def.key] = existing?.value ?? "false";
      }
      setValues(initial);
    }
  }, [settings]);

  const handleToggle = (key: string, checked: boolean) => {
    setValues((prev) => ({ ...prev, [key]: checked ? "true" : "false" }));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      for (const def of KNOWN_SETTINGS) {
        const existing = settings?.find((s) => s.setting_key === def.key);
        const newValue = values[def.key] ?? "false";
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
      }
    } finally {
      setSaving(false);
    }
  };

  const hasChanges = KNOWN_SETTINGS.some((def) => {
    const existing = settings?.find((s) => s.setting_key === def.key);
    const currentValue = values[def.key] ?? "false";
    const savedValue = existing?.value ?? "false";
    return currentValue !== savedValue;
  });

  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold mb-6">Server Settings</h1>

      <div className="flex flex-col gap-6 max-w-2xl">
        {KNOWN_SETTINGS.map((def) => (
          <div key={def.key} className="flex flex-col gap-1">
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
