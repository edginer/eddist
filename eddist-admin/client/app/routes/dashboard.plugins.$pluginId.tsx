import Editor from "@monaco-editor/react";
import { Alert, Badge, Button, Checkbox, Label, TextInput, Textarea } from "flowbite-react";
import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import client from "~/openapi/client";
import { components } from "~/openapi/schema";

type Plugin = components["schemas"]["PluginResponse"];
type PluginHook = components["schemas"]["PluginHook"];

const AVAILABLE_HOOKS: { value: PluginHook; label: string; description: string }[] = [
  {
    value: "before_post_thread",
    label: "Before Post Thread",
    description: "Executed before a new thread is created. Can modify thread data.",
  },
  {
    value: "after_post_thread",
    label: "After Post Thread",
    description: "Executed after a thread is successfully created.",
  },
  {
    value: "before_post_response",
    label: "Before Post Response",
    description: "Executed before a response is posted. Can modify response data.",
  },
  {
    value: "after_post_response",
    label: "After Post Response",
    description: "Executed after a response is successfully posted.",
  },
];

const DEFAULT_SCRIPT = `-- Example plugin script
-- Available hooks: before_post_thread, after_post_thread, before_post_response, after_post_response

function before_post_thread(data)
    -- Modify thread data before creation
    local content = eddist.get_content(data)
    eddist.log("info", "Creating thread with content length: " .. #content)
    return data
end

function after_post_thread(data)
    -- Execute after thread creation
    eddist.log("info", "Thread created successfully")
    return data
end

function before_post_response(data)
    -- Modify response data before posting
    local content = eddist.get_content(data)
    eddist.log("info", "Posting response with content length: " .. #content)
    return data
end

function after_post_response(data)
    -- Execute after response posted
    eddist.log("info", "Response posted successfully")
    return data
end
`;

const PluginEditorPage = () => {
  const { pluginId } = useParams();
  const navigate = useNavigate();
  const isNew = pluginId === "new";

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [script, setScript] = useState(DEFAULT_SCRIPT);
  const [selectedHooks, setSelectedHooks] = useState<PluginHook[]>([]);
  const [enabled, setEnabled] = useState(true);
  const [loading, setLoading] = useState(!isNew);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [successMessage, setSuccessMessage] = useState("");

  const loadPlugin = useCallback(async () => {
    if (isNew) return;

    setLoading(true);
    setError("");

    try {
      const { data, error: fetchError } = await client.GET("/plugins/{id}", {
        params: {
          path: {
            id: parseInt(pluginId!),
          },
        },
      });

      if (fetchError || !data) {
        setError("Failed to load plugin");
        return;
      }

      setName(data.name);
      setDescription(data.description || "");
      setScript(data.script);
      setSelectedHooks(data.hooks);
      setEnabled(data.enabled);
    } catch (err: any) {
      setError(err.message || "Failed to load plugin");
    } finally {
      setLoading(false);
    }
  }, [pluginId, isNew]);

  useEffect(() => {
    loadPlugin();
  }, [loadPlugin]);

  const handleHookToggle = (hook: PluginHook) => {
    setSelectedHooks((prev) =>
      prev.includes(hook) ? prev.filter((h) => h !== hook) : [...prev, hook]
    );
  };

  const handleSave = async () => {
    setError("");
    setSuccessMessage("");

    if (!name.trim()) {
      setError("Plugin name is required");
      return;
    }

    if (selectedHooks.length === 0) {
      setError("At least one hook must be selected");
      return;
    }

    if (!script.trim()) {
      setError("Script is required");
      return;
    }

    setSaving(true);

    try {
      if (isNew) {
        const { data, error: createError } = await client.POST("/plugins", {
          body: {
            name,
            description: description || null,
            script,
            hooks: selectedHooks,
            http_whitelist: null,
          },
        });

        if (createError || !data) {
          setError("Failed to create plugin");
          return;
        }

        setSuccessMessage("Plugin created successfully");
        setTimeout(() => navigate("/dashboard/plugins"), 1500);
      } else {
        const { error: updateError } = await client.PUT("/plugins/{id}", {
          params: {
            path: {
              id: parseInt(pluginId!),
            },
          },
          body: {
            name,
            description: description || null,
            script,
            hooks: selectedHooks,
            http_whitelist: null,
            enabled,
          },
        });

        if (updateError) {
          setError("Failed to update plugin");
          return;
        }

        setSuccessMessage("Plugin updated successfully");
      }
    } catch (err: any) {
      setError(err.message || "Failed to save plugin");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="p-6">
        <div className="text-center py-8">Loading plugin...</div>
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-2xl font-bold">
          {isNew ? "Create New Plugin" : `Edit Plugin: ${name}`}
        </h1>
        <Button color="gray" onClick={() => navigate("/dashboard/plugins")}>
          Back to List
        </Button>
      </div>

      {error && (
        <Alert color="failure" className="mb-4" onDismiss={() => setError("")}>
          {error}
        </Alert>
      )}

      {successMessage && (
        <Alert
          color="success"
          className="mb-4"
          onDismiss={() => setSuccessMessage("")}
        >
          {successMessage}
        </Alert>
      )}

      <div className="space-y-6">
        {/* Basic Info */}
        <div className="bg-white rounded-lg shadow p-6 space-y-4">
          <h2 className="text-lg font-semibold mb-4">Basic Information</h2>

          <div>
            <Label htmlFor="name">Plugin Name *</Label>
            <TextInput
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-plugin"
              required
            />
          </div>

          <div>
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="A brief description of what this plugin does"
              rows={3}
            />
          </div>

          {!isNew && (
            <div className="flex items-center gap-2">
              <Checkbox
                id="enabled"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
              />
              <Label htmlFor="enabled">Plugin Enabled</Label>
            </div>
          )}
        </div>

        {/* Hook Selection */}
        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold mb-4">Hook Selection *</h2>
          <div className="space-y-3">
            {AVAILABLE_HOOKS.map((hook) => (
              <div key={hook.value} className="flex items-start gap-3 p-3 border rounded">
                <Checkbox
                  id={hook.value}
                  checked={selectedHooks.includes(hook.value)}
                  onChange={() => handleHookToggle(hook.value)}
                />
                <div className="flex-1">
                  <Label htmlFor={hook.value} className="font-medium">
                    {hook.label}
                  </Label>
                  <p className="text-sm text-gray-600 mt-1">{hook.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Script Editor */}
        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold mb-4">Lua Script *</h2>
          <div className="mb-4">
            <div className="text-sm text-gray-600 space-y-1">
              <p>Available Eddist API functions:</p>
              <ul className="list-disc list-inside ml-4 space-y-1">
                <li><code className="bg-gray-100 px-1 rounded">eddist.get_content(data)</code> - Get the content/body text</li>
                <li><code className="bg-gray-100 px-1 rounded">eddist.set_content(data, new_content)</code> - Modify content/body</li>
                <li><code className="bg-gray-100 px-1 rounded">eddist.get_author_id(data)</code> - Get author ID (if available)</li>
                <li><code className="bg-gray-100 px-1 rounded">eddist.log(level, message)</code> - Log message (levels: "info", "warn", "error")</li>
              </ul>
            </div>
          </div>
          <div className="border rounded overflow-hidden">
            <Editor
              height="500px"
              defaultLanguage="lua"
              value={script}
              onChange={(value) => setScript(value || "")}
              theme="vs-dark"
              options={{
                minimap: { enabled: false },
                fontSize: 14,
                lineNumbers: "on",
                scrollBeyondLastLine: false,
                automaticLayout: true,
              }}
            />
          </div>
        </div>

        {/* Actions */}
        <div className="flex gap-3">
          <Button onClick={handleSave} disabled={saving}>
            {saving ? "Saving..." : isNew ? "Create Plugin" : "Update Plugin"}
          </Button>
          <Button color="gray" onClick={() => navigate("/dashboard/plugins")}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
};

export default PluginEditorPage;
