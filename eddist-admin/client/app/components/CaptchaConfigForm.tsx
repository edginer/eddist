import {
  Button,
  Checkbox,
  Label,
  Select,
  Textarea,
  TextInput,
} from "flowbite-react";
import { Controller, useForm } from "react-hook-form";
import type { paths } from "~/openapi/schema";

type CaptchaConfig =
  paths["/captcha-configs/"]["get"]["responses"]["200"]["content"]["application/json"][number];
type CreateCaptchaConfigInput =
  paths["/captcha-configs/"]["post"]["requestBody"]["content"]["application/json"];
type UpdateCaptchaConfigInput =
  paths["/captcha-configs/{id}/"]["patch"]["requestBody"]["content"]["application/json"];

const KNOWN_PROVIDERS = ["turnstile", "hcaptcha", "monocle", "custom"];
const FIRST_CLASS_PROVIDERS = ["turnstile", "hcaptcha", "monocle"];

type Props =
  | {
      mode: "create";
      onSubmit: (data: CreateCaptchaConfigInput) => void;
      onReset?: () => void;
    }
  | {
      mode: "edit";
      defaultValues: CaptchaConfig;
      onSubmit: (data: UpdateCaptchaConfigInput) => void;
      onReset?: () => void;
    };

const CaptchaConfigForm = (props: Props) => {
  const { register, handleSubmit, control, watch, reset } = useForm<
    CreateCaptchaConfigInput & UpdateCaptchaConfigInput & { capture_fields?: string | string[] }
  >();

  const defaults = props.mode === "edit" ? props.defaultValues : undefined;
  const isCreate = props.mode === "create";

  const watchProvider = watch("provider");
  const currentProvider = watchProvider ?? defaults?.provider;

  const isFirstClassProvider = (provider: string | undefined) =>
    provider ? FIRST_CLASS_PROVIDERS.includes(provider) : false;

  const showWidgetConfig = !isFirstClassProvider(currentProvider);
  const showVerificationConfig = currentProvider === "custom";

  const transformCaptureFields = (captureFields: string | string[] | undefined) => {
    if (typeof captureFields === "string") {
      return captureFields.trim()
        ? captureFields.split(",").map((s) => s.trim()).filter(Boolean)
        : [];
    }
    return captureFields;
  };

  const onFormSubmit = handleSubmit((data) => {
    const widget =
      data.widget?.form_field_name &&
      data.widget?.script_url &&
      data.widget?.widget_html
        ? data.widget
        : undefined;

    if (isCreate) {
      props.onSubmit({
        ...data,
        capture_fields: transformCaptureFields(data.capture_fields) ?? [],
        widget,
      } as CreateCaptchaConfigInput);
    } else {
      const emptyToUndefined = (v: string | undefined | null) =>
        v?.trim() ? v : undefined;

      props.onSubmit({
        ...data,
        name: emptyToUndefined(data.name),
        provider: emptyToUndefined(data.provider),
        site_key: emptyToUndefined(data.site_key),
        secret: emptyToUndefined(data.secret),
        base_url: emptyToUndefined(data.base_url),
        capture_fields: transformCaptureFields(data.capture_fields),
        widget,
      } as UpdateCaptchaConfigInput);
    }

    reset();
    props.onReset?.();
  });

  return (
    <form onSubmit={onFormSubmit}>
      <div className="flex flex-col gap-4">
        <div>
          <Label>Name</Label>
          <TextInput
            {...register("name", { required: isCreate })}
            defaultValue={defaults?.name}
            placeholder="My Turnstile Config"
            required={isCreate}
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <Label>Provider</Label>
            <Select
              {...register("provider", { required: isCreate })}
              defaultValue={defaults?.provider}
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
              {...register("display_order", { valueAsNumber: true })}
              type="number"
              defaultValue={defaults?.display_order ?? 0}
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <Label>Site Key</Label>
            <TextInput
              {...register("site_key", { required: isCreate })}
              defaultValue={defaults?.site_key}
              placeholder="Site key..."
              required={isCreate}
            />
          </div>
          <div>
            <Label>{isCreate ? "Secret" : "Secret (leave blank to keep existing)"}</Label>
            <TextInput
              {...register("secret", { required: isCreate })}
              type="password"
              placeholder={isCreate ? "Secret key..." : "New secret key..."}
              required={isCreate}
            />
          </div>
        </div>

        <div>
          <Label>Base URL (optional)</Label>
          <TextInput
            {...register("base_url")}
            defaultValue={defaults?.base_url ?? ""}
            placeholder="https://example.com (for self-hosted providers)"
          />
        </div>

        {showWidgetConfig && (
          <div className="border-t pt-4 mt-2">
            <h3 className="font-semibold mb-2">Widget Configuration</h3>
            <p className="text-sm text-gray-500 mb-4">
              Required for custom providers. First-class providers (Turnstile,
              hCaptcha, Monocle) use default configurations.
            </p>
            <div className="flex flex-col gap-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Form Field Name</Label>
                  <TextInput
                    {...register("widget.form_field_name", {
                      required: isCreate && currentProvider === "custom",
                    })}
                    defaultValue={defaults?.widget?.form_field_name}
                    placeholder="cf-turnstile-response"
                    required={isCreate && currentProvider === "custom"}
                  />
                </div>
                <div>
                  <Label>Script URL</Label>
                  <TextInput
                    {...register("widget.script_url", {
                      required: isCreate && currentProvider === "custom",
                    })}
                    defaultValue={defaults?.widget?.script_url}
                    placeholder="https://challenges.cloudflare.com/turnstile/v0/api.js"
                    required={isCreate && currentProvider === "custom"}
                  />
                </div>
              </div>
              <div>
                <Label>Widget HTML</Label>
                <Textarea
                  {...register("widget.widget_html", {
                    required: isCreate && currentProvider === "custom",
                  })}
                  defaultValue={defaults?.widget?.widget_html}
                  placeholder='<div class="cf-turnstile" data-sitekey="{{site_key}}"></div>'
                  rows={3}
                  required={isCreate && currentProvider === "custom"}
                />
              </div>
              <div>
                <Label>Script Handler (optional)</Label>
                <Textarea
                  {...register("widget.script_handler")}
                  defaultValue={defaults?.widget?.script_handler ?? ""}
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
            {...register("capture_fields")}
            defaultValue={defaults?.capture_fields?.join(", ")}
            placeholder="score, vpn, anon"
          />
        </div>

        {showVerificationConfig && (
          <div className="border-t pt-4 mt-2">
            <h3 className="font-semibold mb-2">Verification Configuration</h3>
            <div className="flex flex-col gap-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Verification URL</Label>
                  <TextInput
                    {...register("verification.url")}
                    defaultValue={defaults?.verification?.url}
                    placeholder="{{base_url}}/api/verify"
                  />
                </div>
                <div>
                  <Label>Method</Label>
                  <Select
                    {...register("verification.method")}
                    defaultValue={defaults?.verification?.method}
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
                    {...register("verification.request_format")}
                    defaultValue={defaults?.verification?.request_format}
                  >
                    <option value="Form">Form</option>
                    <option value="Json">JSON</option>
                    <option value="PlainText">Plain Text</option>
                  </Select>
                </div>
                <div>
                  <Label>Success Path</Label>
                  <TextInput
                    {...register("verification.success_path")}
                    defaultValue={
                      defaults?.verification?.success_path ?? "success"
                    }
                    placeholder="success"
                  />
                </div>
              </div>
              <div>
                <Label>Body Template (for PlainText)</Label>
                <Textarea
                  {...register("verification.body_template")}
                  defaultValue={defaults?.verification?.body_template ?? ""}
                  placeholder="{{response}}"
                  rows={2}
                />
              </div>
              <div className="flex gap-4">
                <Controller
                  name="verification.include_ip"
                  control={control}
                  defaultValue={defaults?.verification?.include_ip ?? false}
                  render={({ field }) => (
                    <div className="flex items-center gap-2">
                      <Checkbox
                        id="include_ip"
                        checked={field.value}
                        onChange={field.onChange}
                      />
                      <Label htmlFor="include_ip">Include IP</Label>
                    </div>
                  )}
                />
                <Controller
                  name="verification.negate_success"
                  control={control}
                  defaultValue={
                    defaults?.verification?.negate_success ?? false
                  }
                  render={({ field }) => (
                    <div className="flex items-center gap-2">
                      <Checkbox
                        id="negate_success"
                        checked={field.value}
                        onChange={field.onChange}
                      />
                      <Label htmlFor="negate_success">Negate Success</Label>
                    </div>
                  )}
                />
              </div>
            </div>
          </div>
        )}

        <Controller
          name="is_active"
          control={control}
          defaultValue={defaults?.is_active ?? true}
          render={({ field }) => (
            <div className="flex items-center gap-2">
              <Checkbox
                id="is_active"
                checked={field.value ?? undefined}
                onChange={field.onChange}
              />
              <Label htmlFor="is_active">Active</Label>
            </div>
          )}
        />

        <Button type="submit">{isCreate ? "Create" : "Update"}</Button>
      </div>
    </form>
  );
};

export default CaptchaConfigForm;
