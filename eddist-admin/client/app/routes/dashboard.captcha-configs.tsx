import {
  Badge,
  Button,
  Checkbox,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
  Textarea,
  TextInput,
} from "flowbite-react";
import { useState, useEffect } from "react";
import { FaPlus, FaEdit, FaTrash } from "react-icons/fa";
import { useForm, Controller } from "react-hook-form";
import { toast } from "react-toastify";
import { useQueryClient } from "@tanstack/react-query";
import {
  getCaptchaConfigs,
  createCaptchaConfig,
  updateCaptchaConfig,
  deleteCaptchaConfig,
} from "~/hooks/queries";
import type { paths } from "~/openapi/schema";

type CaptchaConfig =
  paths["/captcha-configs/"]["get"]["responses"]["200"]["content"]["application/json"][number];
type CreateCaptchaConfigInput =
  paths["/captcha-configs/"]["post"]["requestBody"]["content"]["application/json"];
type UpdateCaptchaConfigInput =
  paths["/captcha-configs/{id}/"]["patch"]["requestBody"]["content"]["application/json"];

const KNOWN_PROVIDERS = ["turnstile", "hcaptcha", "monocle", "custom"];
const FIRST_CLASS_PROVIDERS = ["turnstile", "hcaptcha", "monocle"];

const CaptchaConfigs = () => {
  const queryClient = useQueryClient();
  const [openCreateModal, setOpenCreateModal] = useState(false);
  const [openEditModal, setOpenEditModal] = useState(false);
  const [selectedConfig, setSelectedConfig] = useState<
    CaptchaConfig | undefined
  >();

  const {
    register: registerCreate,
    handleSubmit: handleCreateSubmit,
    reset: resetCreate,
    control: controlCreate,
    watch: watchCreate,
  } = useForm<CreateCaptchaConfigInput>();

  const {
    register: registerEdit,
    handleSubmit: handleEditSubmit,
    reset: resetEdit,
    control: controlEdit,
    watch: watchEdit,
  } = useForm<UpdateCaptchaConfigInput>();

  const watchProvider = watchCreate("provider");
  const watchEditProvider = watchEdit("provider");

  const isFirstClassProvider = (provider: string | undefined) =>
    provider ? FIRST_CLASS_PROVIDERS.includes(provider) : false;

  const showWidgetConfig = !isFirstClassProvider(watchProvider);
  const showEditWidgetConfig = !isFirstClassProvider(watchEditProvider ?? selectedConfig?.provider);

  const { data: configs } = getCaptchaConfigs();

  const handleDelete = async (id: string) => {
    if (
      window.confirm("Are you sure you want to delete this captcha config?")
    ) {
      try {
        await deleteCaptchaConfig({ params: { path: { id } } }).mutate();
        await queryClient.invalidateQueries({
          queryKey: ["/captcha-configs/"],
        });
        toast.success("Captcha config deleted successfully");
      } catch {
        toast.error("Failed to delete captcha config");
      }
    }
  };

  const onCreateSubmit = async (data: CreateCaptchaConfigInput & { capture_fields?: string | string[] }) => {
    try {
      // Transform capture_fields from comma-separated string to array
      const captureFieldsStr = data.capture_fields;

      // Only include widget if all required fields are present (custom provider)
      const widget = data.widget?.form_field_name && data.widget?.script_url && data.widget?.widget_html
        ? data.widget
        : undefined;

      const transformedData = {
        ...data,
        capture_fields: typeof captureFieldsStr === "string" && captureFieldsStr.trim()
          ? captureFieldsStr.split(",").map((s) => s.trim()).filter(Boolean)
          : [],
        widget,
      };

      await createCaptchaConfig({ body: transformedData }).mutate();
      await queryClient.invalidateQueries({ queryKey: ["/captcha-configs/"] });
      toast.success("Captcha config created successfully");
      setOpenCreateModal(false);
      resetCreate();
    } catch (error: any) {
      const message = error?.message || "Failed to create captcha config";
      toast.error(message);
    }
  };

  const onEditSubmit = async (data: UpdateCaptchaConfigInput & { capture_fields?: string | string[] }) => {
    if (selectedConfig) {
      try {
        // Transform capture_fields from comma-separated string to array
        const captureFieldsStr = data.capture_fields;

        // Only include widget if all required fields are present (custom provider)
        const widget = data.widget?.form_field_name && data.widget?.script_url && data.widget?.widget_html
          ? data.widget
          : undefined;

        const transformedData = {
          ...data,
          capture_fields: typeof captureFieldsStr === "string"
            ? captureFieldsStr.trim()
              ? captureFieldsStr.split(",").map((s) => s.trim()).filter(Boolean)
              : []
            : captureFieldsStr,
          widget,
        };

        await updateCaptchaConfig({
          params: { path: { id: selectedConfig.id } },
          body: transformedData,
        }).mutate();
        await queryClient.invalidateQueries({
          queryKey: ["/captcha-configs/"],
        });
        toast.success("Captcha config updated successfully");
        setOpenEditModal(false);
        resetEdit();
      } catch (error: any) {
        const message = error?.message || "Failed to update captcha config";
        toast.error(message);
      }
    }
  };

  const getProviderBadgeColor = (provider: string) => {
    switch (provider.toLowerCase()) {
      case "turnstile":
        return "warning";
      case "hcaptcha":
        return "info";
      case "monocle":
        return "purple";
      default:
        return "gray";
    }
  };

  return (
    <>
      <div className="p-4">
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-2xl font-bold">Captcha Configs</h1>
          <Button onClick={() => setOpenCreateModal(true)}>
            <FaPlus className="mr-2" />
            Create Config
          </Button>
        </div>

        <Table>
          <TableHead>
            <TableHeadCell>Name</TableHeadCell>
            <TableHeadCell>Provider</TableHeadCell>
            <TableHeadCell>Site Key</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell>Order</TableHeadCell>
            <TableHeadCell>Actions</TableHeadCell>
          </TableHead>
          <TableBody>
            {configs?.map((config) => (
              <TableRow className="border-gray-200" key={config.id}>
                <TableCell>{config.name}</TableCell>
                <TableCell>
                  <Badge color={getProviderBadgeColor(config.provider)}>
                    {config.provider}
                  </Badge>
                </TableCell>
                <TableCell>
                  <code className="text-sm text-gray-600 max-w-xs truncate block">
                    {config.site_key}
                  </code>
                </TableCell>
                <TableCell>
                  <Badge color={config.is_active ? "success" : "gray"}>
                    {config.is_active ? "Active" : "Inactive"}
                  </Badge>
                </TableCell>
                <TableCell>{config.display_order}</TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button
                      size="xs"
                      onClick={() => {
                        setSelectedConfig(config);
                        setOpenEditModal(true);
                      }}
                    >
                      <FaEdit />
                    </Button>
                    <Button
                      size="xs"
                      color="alternative"
                      onClick={() => handleDelete(config.id)}
                    >
                      <FaTrash />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* Create Modal */}
      <Modal
        show={openCreateModal}
        onClose={() => setOpenCreateModal(false)}
        size="xl"
      >
        <ModalHeader className="border-gray-200">
          Create Captcha Config
        </ModalHeader>
        <ModalBody>
          <form onSubmit={handleCreateSubmit(onCreateSubmit)}>
            <div className="flex flex-col gap-4">
              <div>
                <Label>Name</Label>
                <TextInput
                  {...registerCreate("name", { required: true })}
                  placeholder="My Turnstile Config"
                  required
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Provider</Label>
                  <Select {...registerCreate("provider", { required: true })}>
                    <option value="">Select provider...</option>
                    {KNOWN_PROVIDERS.map((p) => (
                      <option key={p} value={p}>
                        {p}
                      </option>
                    ))}
                  </Select>
                </div>
                <div>
                  <Label>Display Order</Label>
                  <TextInput
                    {...registerCreate("display_order", { valueAsNumber: true })}
                    type="number"
                    defaultValue={0}
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Site Key</Label>
                  <TextInput
                    {...registerCreate("site_key", { required: true })}
                    placeholder="Site key..."
                    required
                  />
                </div>
                <div>
                  <Label>Secret</Label>
                  <TextInput
                    {...registerCreate("secret", { required: true })}
                    type="password"
                    placeholder="Secret key..."
                    required
                  />
                </div>
              </div>

              <div>
                <Label>Base URL (optional)</Label>
                <TextInput
                  {...registerCreate("base_url")}
                  placeholder="https://example.com (for self-hosted providers)"
                />
              </div>

              {showWidgetConfig && (
                <div className="border-t pt-4 mt-2">
                  <h3 className="font-semibold mb-2">Widget Configuration</h3>
                  <p className="text-sm text-gray-500 mb-4">
                    Required for custom providers. First-class providers (Turnstile, hCaptcha, Monocle) use default configurations.
                  </p>
                  <div className="flex flex-col gap-4">
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <Label>Form Field Name</Label>
                        <TextInput
                          {...registerCreate("widget.form_field_name", {
                            required: watchProvider === "custom",
                          })}
                          placeholder="cf-turnstile-response"
                          required={watchProvider === "custom"}
                        />
                      </div>
                      <div>
                        <Label>Script URL</Label>
                        <TextInput
                          {...registerCreate("widget.script_url", {
                            required: watchProvider === "custom",
                          })}
                          placeholder="https://challenges.cloudflare.com/turnstile/v0/api.js"
                          required={watchProvider === "custom"}
                        />
                      </div>
                    </div>
                    <div>
                      <Label>Widget HTML</Label>
                      <Textarea
                        {...registerCreate("widget.widget_html", {
                          required: watchProvider === "custom",
                        })}
                        placeholder='<div class="cf-turnstile" data-sitekey="{{site_key}}"></div>'
                        rows={3}
                        required={watchProvider === "custom"}
                      />
                    </div>
                    <div>
                      <Label>Script Handler (optional)</Label>
                      <Textarea
                        {...registerCreate("widget.script_handler")}
                        placeholder="JavaScript code for event handling..."
                        rows={2}
                      />
                    </div>
                  </div>
                </div>
              )}

              <div>
                <Label>Capture Fields (comma-separated)</Label>
                <TextInput
                  {...registerCreate("capture_fields")}
                  placeholder="score, vpn, anon"
                />
              </div>

              {watchProvider === "custom" && (
                <div className="border-t pt-4 mt-2">
                  <h3 className="font-semibold mb-2">
                    Verification Configuration
                  </h3>
                  <div className="flex flex-col gap-4">
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <Label>Verification URL</Label>
                        <TextInput
                          {...registerCreate("verification.url")}
                          placeholder="{{base_url}}/api/verify"
                        />
                      </div>
                      <div>
                        <Label>Method</Label>
                        <Select {...registerCreate("verification.method")}>
                          <option value="Post">POST</option>
                          <option value="Get">GET</option>
                        </Select>
                      </div>
                    </div>
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <Label>Request Format</Label>
                        <Select
                          {...registerCreate("verification.request_format")}
                        >
                          <option value="Form">Form</option>
                          <option value="Json">JSON</option>
                          <option value="PlainText">Plain Text</option>
                        </Select>
                      </div>
                      <div>
                        <Label>Success Path</Label>
                        <TextInput
                          {...registerCreate("verification.success_path")}
                          placeholder="success"
                          defaultValue="success"
                        />
                      </div>
                    </div>
                    <div>
                      <Label>Body Template (for PlainText)</Label>
                      <Textarea
                        {...registerCreate("verification.body_template")}
                        placeholder="{{response}}"
                        rows={2}
                      />
                    </div>
                    <div className="flex gap-4">
                      <Controller
                        name="verification.include_ip"
                        control={controlCreate}
                        render={({ field }) => (
                          <div className="flex items-center gap-2">
                            <Checkbox
                              id="include_ip_create"
                              checked={field.value}
                              onChange={field.onChange}
                            />
                            <Label htmlFor="include_ip_create">
                              Include IP
                            </Label>
                          </div>
                        )}
                      />
                      <Controller
                        name="verification.negate_success"
                        control={controlCreate}
                        render={({ field }) => (
                          <div className="flex items-center gap-2">
                            <Checkbox
                              id="negate_success_create"
                              checked={field.value}
                              onChange={field.onChange}
                            />
                            <Label htmlFor="negate_success_create">
                              Negate Success
                            </Label>
                          </div>
                        )}
                      />
                    </div>
                  </div>
                </div>
              )}

              <Controller
                name="is_active"
                control={controlCreate}
                defaultValue={true}
                render={({ field }) => (
                  <div className="flex items-center gap-2">
                    <Checkbox
                      id="is_active_create"
                      checked={field.value}
                      onChange={field.onChange}
                    />
                    <Label htmlFor="is_active_create">Active</Label>
                  </div>
                )}
              />

              <Button type="submit">Create</Button>
            </div>
          </form>
        </ModalBody>
      </Modal>

      {/* Edit Modal */}
      <Modal
        show={openEditModal}
        onClose={() => setOpenEditModal(false)}
        size="xl"
      >
        <ModalHeader className="border-gray-200">
          Edit Captcha Config
        </ModalHeader>
        <ModalBody>
          <form onSubmit={handleEditSubmit(onEditSubmit)}>
            <div className="flex flex-col gap-4">
              <div>
                <Label>Name</Label>
                <TextInput
                  {...registerEdit("name")}
                  defaultValue={selectedConfig?.name}
                  placeholder="My Turnstile Config"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Provider</Label>
                  <Select
                    {...registerEdit("provider")}
                    defaultValue={selectedConfig?.provider}
                  >
                    <option value="">Select provider...</option>
                    {KNOWN_PROVIDERS.map((p) => (
                      <option key={p} value={p}>
                        {p}
                      </option>
                    ))}
                  </Select>
                </div>
                <div>
                  <Label>Display Order</Label>
                  <TextInput
                    {...registerEdit("display_order", { valueAsNumber: true })}
                    type="number"
                    defaultValue={selectedConfig?.display_order}
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Site Key</Label>
                  <TextInput
                    {...registerEdit("site_key")}
                    defaultValue={selectedConfig?.site_key}
                    placeholder="Site key..."
                  />
                </div>
                <div>
                  <Label>Secret (leave blank to keep existing)</Label>
                  <TextInput
                    {...registerEdit("secret")}
                    type="password"
                    placeholder="New secret key..."
                  />
                </div>
              </div>

              <div>
                <Label>Base URL (optional)</Label>
                <TextInput
                  {...registerEdit("base_url")}
                  defaultValue={selectedConfig?.base_url ?? ""}
                  placeholder="https://example.com (for self-hosted providers)"
                />
              </div>

              {showEditWidgetConfig && (
                <div className="border-t pt-4 mt-2">
                  <h3 className="font-semibold mb-2">Widget Configuration</h3>
                  <p className="text-sm text-gray-500 mb-4">
                    Required for custom providers. First-class providers (Turnstile, hCaptcha, Monocle) use default configurations.
                  </p>
                  <div className="flex flex-col gap-4">
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <Label>Form Field Name</Label>
                        <TextInput
                          {...registerEdit("widget.form_field_name")}
                          defaultValue={selectedConfig?.widget?.form_field_name}
                          placeholder="cf-turnstile-response"
                        />
                      </div>
                      <div>
                        <Label>Script URL</Label>
                        <TextInput
                          {...registerEdit("widget.script_url")}
                          defaultValue={selectedConfig?.widget?.script_url}
                          placeholder="https://challenges.cloudflare.com/turnstile/v0/api.js"
                        />
                      </div>
                    </div>
                    <div>
                      <Label>Widget HTML</Label>
                      <Textarea
                        {...registerEdit("widget.widget_html")}
                        defaultValue={selectedConfig?.widget?.widget_html}
                        placeholder='<div class="cf-turnstile" data-sitekey="{{site_key}}"></div>'
                        rows={3}
                      />
                    </div>
                    <div>
                      <Label>Script Handler (optional)</Label>
                      <Textarea
                        {...registerEdit("widget.script_handler")}
                        defaultValue={
                          selectedConfig?.widget?.script_handler ?? ""
                        }
                        placeholder="JavaScript code for event handling..."
                        rows={2}
                      />
                    </div>
                  </div>
                </div>
              )}

              <div>
                <Label>Capture Fields (comma-separated)</Label>
                <TextInput
                  {...registerEdit("capture_fields")}
                  defaultValue={selectedConfig?.capture_fields?.join(", ")}
                  placeholder="score, vpn, anon"
                />
              </div>

              {(selectedConfig?.provider === "custom" ||
                watchEditProvider === "custom") && (
                <div className="border-t pt-4 mt-2">
                  <h3 className="font-semibold mb-2">
                    Verification Configuration
                  </h3>
                  <div className="flex flex-col gap-4">
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <Label>Verification URL</Label>
                        <TextInput
                          {...registerEdit("verification.url")}
                          defaultValue={selectedConfig?.verification?.url}
                          placeholder="{{base_url}}/api/verify"
                        />
                      </div>
                      <div>
                        <Label>Method</Label>
                        <Select
                          {...registerEdit("verification.method")}
                          defaultValue={selectedConfig?.verification?.method}
                        >
                          <option value="Post">POST</option>
                          <option value="Get">GET</option>
                        </Select>
                      </div>
                    </div>
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <Label>Request Format</Label>
                        <Select
                          {...registerEdit("verification.request_format")}
                          defaultValue={
                            selectedConfig?.verification?.request_format
                          }
                        >
                          <option value="Form">Form</option>
                          <option value="Json">JSON</option>
                          <option value="PlainText">Plain Text</option>
                        </Select>
                      </div>
                      <div>
                        <Label>Success Path</Label>
                        <TextInput
                          {...registerEdit("verification.success_path")}
                          defaultValue={
                            selectedConfig?.verification?.success_path ??
                            "success"
                          }
                          placeholder="success"
                        />
                      </div>
                    </div>
                    <div>
                      <Label>Body Template (for PlainText)</Label>
                      <Textarea
                        {...registerEdit("verification.body_template")}
                        defaultValue={
                          selectedConfig?.verification?.body_template ?? ""
                        }
                        placeholder="{{response}}"
                        rows={2}
                      />
                    </div>
                    <div className="flex gap-4">
                      <Controller
                        name="verification.include_ip"
                        control={controlEdit}
                        defaultValue={
                          selectedConfig?.verification?.include_ip ?? false
                        }
                        render={({ field }) => (
                          <div className="flex items-center gap-2">
                            <Checkbox
                              id="include_ip_edit"
                              checked={field.value}
                              onChange={field.onChange}
                            />
                            <Label htmlFor="include_ip_edit">Include IP</Label>
                          </div>
                        )}
                      />
                      <Controller
                        name="verification.negate_success"
                        control={controlEdit}
                        defaultValue={
                          selectedConfig?.verification?.negate_success ?? false
                        }
                        render={({ field }) => (
                          <div className="flex items-center gap-2">
                            <Checkbox
                              id="negate_success_edit"
                              checked={field.value}
                              onChange={field.onChange}
                            />
                            <Label htmlFor="negate_success_edit">
                              Negate Success
                            </Label>
                          </div>
                        )}
                      />
                    </div>
                  </div>
                </div>
              )}

              <Controller
                name="is_active"
                control={controlEdit}
                defaultValue={selectedConfig?.is_active}
                render={({ field }) => (
                  <div className="flex items-center gap-2">
                    <Checkbox
                      id="is_active_edit"
                      checked={field.value}
                      onChange={field.onChange}
                    />
                    <Label htmlFor="is_active_edit">Active</Label>
                  </div>
                )}
              />

              <Button type="submit">Update</Button>
            </div>
          </form>
        </ModalBody>
      </Modal>
    </>
  );
};

export default CaptchaConfigs;
