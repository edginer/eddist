import { Button, Label, Textarea, Spinner } from "flowbite-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "react-toastify";
import { useQueryClient } from "@tanstack/react-query";
import { FaSave, FaExternalLinkAlt } from "react-icons/fa";
import { getTerms, updateTerms } from "~/hooks/queries";
import type { paths } from "~/openapi/schema";

type Terms =
  paths["/terms/"]["get"]["responses"]["200"]["content"]["application/json"];
type UpdateTermsInput =
  paths["/terms/"]["put"]["requestBody"]["content"]["application/json"];

const TermsPage = () => {
  const queryClient = useQueryClient();
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

  const onSubmit = async (data: UpdateTermsInput) => {
    try {
      await updateTerms({ body: data }).mutate();
      await queryClient.invalidateQueries({ queryKey: ["/terms/"] });
      toast.success("Terms updated successfully");
      setIsDirty(false);
    } catch (error: any) {
      const message = error?.message || "Failed to update terms";
      toast.error(message);
    }
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
          <div>Last updated: {new Date(terms.updated_at).toLocaleString()}</div>
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
