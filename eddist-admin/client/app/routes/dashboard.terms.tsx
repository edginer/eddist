import { Button, Label, Textarea, Spinner } from "flowbite-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { FaSave } from "react-icons/fa";
import { getTerms, useUpdateTerms } from "~/hooks/queries";
import type { paths } from "~/openapi/schema";
import { formatDateTime } from "~/utils/format";

type UpdateTermsInput =
  paths["/terms/"]["put"]["requestBody"]["content"]["application/json"];

const TermsPage = () => {
  const [isDirty, setIsDirty] = useState(false);

  const { register, handleSubmit, reset, watch } = useForm<UpdateTermsInput>();

  const { data: terms, isLoading } = getTerms();
  const contentValue = watch("content");

  useEffect(() => {
    if (terms?.content) {
      reset({ content: terms.content });
    }
  }, [terms, reset]);

  useEffect(() => {
    if (terms?.content && contentValue !== undefined) {
      setIsDirty(contentValue !== terms.content);
    }
  }, [contentValue, terms]);

  const updateMutation = useUpdateTerms();

  const onSubmit = (data: UpdateTermsInput) => {
    updateMutation.mutate(
      { body: data },
      {
        onSuccess: () => {
          setIsDirty(false);
        },
      },
    );
  };

  if (isLoading) {
    return (
      <div className="p-4 flex justify-center items-center">
        <Spinner size="lg" />
      </div>
    );
  }

  return (
    <div className="p-4">
      {terms && (
        <div className="mb-4 text-sm text-gray-600">
          <div>Last updated: {formatDateTime(terms.updated_at)}</div>
          {terms.updated_by && <div>Updated by: {terms.updated_by}</div>}
        </div>
      )}

      <form onSubmit={handleSubmit(onSubmit)}>
        <div className="flex flex-col gap-4">
          <div>
            <Label htmlFor="content">Content (Markdown)</Label>
            <Textarea
              {...register("content", { required: true })}
              id="content"
              rows={30}
              placeholder="Enter terms content in markdown format..."
              className="font-mono text-sm"
              required
            />
          </div>

          <div className="flex justify-end gap-2">
            <Button type="submit" disabled={!isDirty}>
              <FaSave className="mr-2" />
              Save Changes
            </Button>
          </div>
        </div>
      </form>
    </div>
  );
};

export default TermsPage;
