import { Alert, Badge, Button, Table } from "flowbite-react";
import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router";
import client from "~/openapi/client";
import { components } from "~/openapi/schema";

type Plugin = components["schemas"]["PluginResponse"];

const PluginsPage = () => {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [successMessage, setSuccessMessage] = useState("");

  const loadPlugins = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const { data, error: fetchError } = await client.GET("/plugins");

      if (fetchError || !data) {
        setError("Failed to load plugins");
        return;
      }

      setPlugins(data.plugins);
    } catch (err: any) {
      setError(err.message || "Failed to load plugins");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPlugins();
  }, [loadPlugins]);

  const handleToggle = async (pluginId: number, currentEnabled: boolean) => {
    try {
      const { error } = await client.PUT("/plugins/{id}/toggle", {
        params: {
          path: {
            id: pluginId,
          },
        },
        body: {
          enabled: !currentEnabled,
        },
      });

      if (error) {
        setError("Failed to toggle plugin");
        return;
      }

      setSuccessMessage(
        `Plugin ${!currentEnabled ? "enabled" : "disabled"} successfully`
      );
      loadPlugins();
    } catch (err: any) {
      setError(err.message || "Failed to toggle plugin");
    }
  };

  const handleDelete = async (pluginId: number, pluginName: string) => {
    if (
      !confirm(
        `Are you sure you want to delete the plugin "${pluginName}"? This action cannot be undone.`
      )
    ) {
      return;
    }

    try {
      const { error } = await client.DELETE("/plugins/{id}", {
        params: {
          path: {
            id: pluginId,
          },
        },
      });

      if (error) {
        setError("Failed to delete plugin");
        return;
      }

      setSuccessMessage("Plugin deleted successfully");
      loadPlugins();
    } catch (err: any) {
      setError(err.message || "Failed to delete plugin");
    }
  };

  return (
    <div className="p-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-2xl font-bold">Plugins</h1>
        <Link to="/dashboard/plugins/new">
          <Button>Create New Plugin</Button>
        </Link>
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

      {loading ? (
        <div className="text-center py-8">Loading plugins...</div>
      ) : plugins.length === 0 ? (
        <div className="text-center py-8 text-gray-500">
          No plugins found. Create your first plugin to get started.
        </div>
      ) : (
        <Table>
          <Table.Head>
            <Table.HeadCell>Name</Table.HeadCell>
            <Table.HeadCell>Description</Table.HeadCell>
            <Table.HeadCell>Hooks</Table.HeadCell>
            <Table.HeadCell>Status</Table.HeadCell>
            <Table.HeadCell>Created</Table.HeadCell>
            <Table.HeadCell>Actions</Table.HeadCell>
          </Table.Head>
          <Table.Body className="divide-y">
            {plugins.map((plugin) => (
              <Table.Row key={plugin.id} className="bg-white">
                <Table.Cell className="font-medium text-gray-900">
                  {plugin.name}
                </Table.Cell>
                <Table.Cell>
                  {plugin.description || <span className="text-gray-400">—</span>}
                </Table.Cell>
                <Table.Cell>
                  <div className="flex flex-wrap gap-1">
                    {plugin.hooks.map((hook) => (
                      <Badge key={hook} color="info" size="sm">
                        {hook}
                      </Badge>
                    ))}
                  </div>
                </Table.Cell>
                <Table.Cell>
                  <Badge color={plugin.enabled ? "success" : "gray"}>
                    {plugin.enabled ? "Enabled" : "Disabled"}
                  </Badge>
                </Table.Cell>
                <Table.Cell>
                  {new Date(plugin.created_at).toLocaleDateString()}
                </Table.Cell>
                <Table.Cell>
                  <div className="flex gap-2">
                    <Link to={`/dashboard/plugins/${plugin.id}`}>
                      <Button size="xs" color="gray">
                        Edit
                      </Button>
                    </Link>
                    <Button
                      size="xs"
                      color={plugin.enabled ? "warning" : "success"}
                      onClick={() => handleToggle(plugin.id, plugin.enabled)}
                    >
                      {plugin.enabled ? "Disable" : "Enable"}
                    </Button>
                    <Button
                      size="xs"
                      color="failure"
                      onClick={() => handleDelete(plugin.id, plugin.name)}
                    >
                      Delete
                    </Button>
                  </div>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}
    </div>
  );
};

export default PluginsPage;
