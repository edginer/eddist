import { Button, Label, Textarea, TextInput } from "flowbite-react";
import { FaSync } from "react-icons/fa";
import { useForm } from "react-hook-form";
import type { paths } from "~/openapi/schema";

type NoticeFormData =
  paths["/notices/"]["post"]["requestBody"]["content"]["application/json"];

function generateSlug(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, "-")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

interface DefaultValues {
  title: string;
  slug: string;
  content: string;
  published_at: string;
}

type Props =
  | {
      mode: "create";
      onSubmit: (data: NoticeFormData) => void;
    }
  | {
      mode: "edit";
      defaultValues: DefaultValues;
      onSubmit: (data: Partial<NoticeFormData>) => void;
    };

const NoticeForm = (props: Props) => {
  const defaults = props.mode === "edit" ? props.defaultValues : undefined;
  const isCreate = props.mode === "create";

  const { register, handleSubmit, setValue, watch, reset } = useForm<
    NoticeFormData | Partial<NoticeFormData>
  >();

  return (
    <form
      onSubmit={handleSubmit((data) => {
        // Convert datetime-local to NaiveDateTime format
        const formattedData = {
          ...data,
          ...(data.published_at && {
            published_at: new Date(data.published_at)
              .toISOString()
              .slice(0, 19),
          }),
        };
        props.onSubmit(formattedData as NoticeFormData & Partial<NoticeFormData>);
        reset();
      })}
    >
      <div className="flex flex-col gap-4">
        <div>
          <Label>Title</Label>
          <TextInput
            {...register("title", { required: isCreate })}
            defaultValue={defaults?.title}
            placeholder="Notice title..."
            required={isCreate}
          />
        </div>
        <div>
          <Label>Slug</Label>
          <div className="flex gap-2">
            <TextInput
              {...register("slug", { required: isCreate })}
              defaultValue={defaults?.slug}
              placeholder="url-friendly-slug"
              required={isCreate}
              className="flex-1"
            />
            <Button
              type="button"
              color="gray"
              size="sm"
              className="items-center"
              onClick={() => {
                const title = watch("title") || defaults?.title;
                if (title) {
                  setValue("slug", generateSlug(title));
                }
              }}
            >
              <FaSync className="m-1 ml-0 mr-2" />
              <span>Generate</span>
            </Button>
          </div>
        </div>
        <div>
          <Label>Content</Label>
          <Textarea
            {...register("content", { required: isCreate })}
            defaultValue={defaults?.content}
            placeholder="Notice content..."
            required={isCreate}
            rows={6}
          />
        </div>
        <div>
          <Label>Published At</Label>
          <TextInput
            {...register("published_at", { required: isCreate })}
            type="datetime-local"
            required={isCreate}
            defaultValue={
              defaults?.published_at
                ? new Date(defaults.published_at).toISOString().slice(0, 16)
                : ""
            }
          />
        </div>
        <Button type="submit">{isCreate ? "Create" : "Update"}</Button>
      </div>
    </form>
  );
};

export default NoticeForm;
