import {
  Accordion,
  AccordionContent,
  AccordionPanel,
  AccordionTitle,
  Button,
  HelperText,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  Textarea,
  TextInput,
} from "flowbite-react";
import { useForm } from "react-hook-form";
import { createBoard, getBoards } from "~/hooks/queries";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { components } from "~/openapi/schema";
import { toast } from "react-toastify";

interface CreateBoardModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  refetch: () => Promise<unknown>;
}

const boardCreationSchema = z.object({
  name: z.string().min(1).max(64),
  board_key: z
    .string()
    .min(1)
    .max(64)
    .regex(/^[a-z0-9\-_]+$/, {
      message: "Board key must be lower alphanumeric or contain - or _",
    })
    // Does not allow `test`, `api`, `auth`, `auth-code`, `bbsmenu` etc.
    .refine(
      (value) => {
        return !["test", "api", "auth", "auth-code", "bbsmenu"].includes(value);
      },
      {
        message:
          "Sorry, this board key is reserved. Please choose another one.",
      }
    ),
  default_name: z.string().min(1).max(64),
  local_rule: z.string().min(1),
  base_thread_creation_span_sec: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  base_response_creation_span_sec: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  max_thread_name_byte_length: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  max_author_name_byte_length: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  max_email_byte_length: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  max_response_body_byte_length: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  max_response_body_lines: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
  threads_archive_cron: z.string().optional(),
  threads_archive_trigger_thread_count: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
});

const CreateBoardModal = ({
  open,
  setOpen,
  refetch,
}: CreateBoardModalProps) => {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<components["schemas"]["CreateBoardInput"]>({
    resolver: zodResolver(boardCreationSchema),
  });

  return (
    <Modal show={open} onClose={() => setOpen(false)}>
      <ModalHeader>Create Board</ModalHeader>

      <ModalBody>
        <form
          onSubmit={handleSubmit(async (data) => {
            try {
              const { mutate } = createBoard({
                body: data,
              });
              await mutate();
              setOpen(false);
              toast.success("Successfully created board");
              await refetch();
            } catch {
              toast.error("Failed to create board");
            }
          })}
        >
          <div className="flex flex-col">
            <Label>Name</Label>
            <TextInput
              placeholder="Name..."
              required
              {...register("name")}
              color={errors.name ? "red" : undefined}
            />
            <HelperText>{errors.name && <>{errors.name.message}</>}</HelperText>
          </div>
          <div className="flex flex-col mt-4">
            <Label>Board Key</Label>
            <TextInput
              placeholder="Board Key..."
              required
              {...register("board_key")}
              color={errors.board_key ? "red" : undefined}
            />
            <HelperText>
              {errors.board_key && <>{errors.board_key.message}</>}
            </HelperText>
          </div>
          <div className="flex flex-col mt-4">
            <Label>Default Name</Label>
            <TextInput
              placeholder="Default Name..."
              required
              {...register("default_name")}
              color={errors.default_name ? "red" : undefined}
            />
            <HelperText>
              {errors.default_name && <>{errors.default_name.message}</>}
            </HelperText>
            <div className="flex flex-col mt-4">
              <Label>Local Rules</Label>
              <Textarea
                placeholder="Local Rules..."
                rows={7}
                required
                {...register("local_rule")}
                color={errors.local_rule ? "red" : undefined}
              />
              <HelperText>
                {errors.local_rule && <>{errors.local_rule.message}</>}
              </HelperText>
            </div>
          </div>
          <Accordion className="mt-4" collapseAll>
            <AccordionPanel title="Advanced Settings">
              <AccordionTitle className="p-3.5">
                Advanced Settings
              </AccordionTitle>
              <AccordionContent>
                <div className="flex flex-col">
                  <div className="flex flex-row content-between space-x-4">
                    <div className="flex flex-col grow">
                      <Label>Base Thread Creation Span (sec)</Label>
                      <TextInput
                        type="number"
                        placeholder="Base Thread Creation Span..."
                        {...register("base_thread_creation_span_sec", {
                          valueAsNumber: true,
                        })}
                        color={
                          errors.base_thread_creation_span_sec
                            ? "red"
                            : undefined
                        }
                      />
                      <HelperText>
                        {errors.base_thread_creation_span_sec && (
                          <>{errors.base_thread_creation_span_sec.message}</>
                        )}
                      </HelperText>
                    </div>
                    <div className="flex flex-col grow">
                      <Label>Base Response Creation Span (sec)</Label>
                      <TextInput
                        type="number"
                        placeholder="Base Response Creation Span..."
                        {...register("base_response_creation_span_sec", {
                          valueAsNumber: true,
                        })}
                        color={
                          errors.base_response_creation_span_sec
                            ? "red"
                            : undefined
                        }
                      />
                      <HelperText>
                        {errors.base_response_creation_span_sec && (
                          <>{errors.base_response_creation_span_sec.message}</>
                        )}
                      </HelperText>
                    </div>
                  </div>
                  <div className="flex flex-row content-between space-x-4 mt-4">
                    <div className="flex flex-col grow">
                      <Label>Max Thread Name Byte Length</Label>
                      <TextInput
                        type="number"
                        placeholder="Max Thread Name Byte Length..."
                        {...register("max_thread_name_byte_length", {
                          valueAsNumber: true,
                        })}
                        color={
                          errors.max_thread_name_byte_length ? "red" : undefined
                        }
                      />
                      <HelperText>
                        {errors.max_thread_name_byte_length && (
                          <>{errors.max_thread_name_byte_length.message}</>
                        )}
                      </HelperText>
                    </div>
                    <div className="flex flex-col grow">
                      <Label>Max Author Name Byte Length</Label>
                      <TextInput
                        type="number"
                        placeholder="Max Author Name Byte Length..."
                        {...register("max_author_name_byte_length", {
                          valueAsNumber: true,
                        })}
                        color={
                          errors.max_author_name_byte_length ? "red" : undefined
                        }
                      />
                      <HelperText>
                        {errors.max_author_name_byte_length && (
                          <>{errors.max_author_name_byte_length.message}</>
                        )}
                      </HelperText>
                    </div>
                  </div>
                </div>
                <div className="flex flex-row content-between space-x-4 mt-4">
                  <div className="flex flex-col grow">
                    <Label>Max Email Byte Length</Label>
                    <TextInput
                      type="number"
                      placeholder="Max Email Byte Length..."
                      {...register("max_email_byte_length", {
                        valueAsNumber: true,
                      })}
                      color={errors.max_email_byte_length ? "red" : undefined}
                    />
                    <HelperText>
                      {errors.max_email_byte_length && (
                        <>{errors.max_email_byte_length.message}</>
                      )}
                    </HelperText>
                  </div>
                  <div className="flex flex-col grow">
                    <Label>Max Response Body Byte Length</Label>
                    <TextInput
                      type="number"
                      placeholder="Max Response Body Byte Length..."
                      {...register("max_response_body_byte_length", {
                        valueAsNumber: true,
                      })}
                      color={
                        errors.max_response_body_byte_length ? "red" : undefined
                      }
                    />
                  </div>
                  <HelperText>
                    {errors.max_response_body_byte_length && (
                      <>{errors.max_response_body_byte_length.message}</>
                    )}
                  </HelperText>
                </div>
                <div className="flex flex-col mt-4">
                  <Label>Max Response Body Lines</Label>
                  <TextInput
                    type="number"
                    placeholder="Max Response Body Lines..."
                    {...register("max_response_body_lines", {
                      valueAsNumber: true,
                    })}
                    color={errors.max_response_body_lines ? "red" : undefined}
                  />
                  <HelperText>
                    {errors.max_response_body_lines && (
                      <>{errors.max_response_body_lines.message}</>
                    )}
                  </HelperText>
                </div>
                <div className="flex flex-row content-between space-x-4 mt-4">
                  <div className="flex flex-col grow">
                    <Label>Threads Archive Cron String</Label>
                    <TextInput
                      placeholder="Threads Archive Cron String..."
                      {...register("threads_archive_cron")}
                      color={errors.threads_archive_cron ? "red" : undefined}
                    />
                    <HelperText>
                      {errors.threads_archive_cron && (
                        <>{errors.threads_archive_cron.message}</>
                      )}
                    </HelperText>
                  </div>
                  <div className="flex flex-col grow">
                    <Label>Threads Archive Trigger Thread Count</Label>
                    <TextInput
                      type="number"
                      placeholder="Threads Archive Trigger Thread Count..."
                      {...register("threads_archive_trigger_thread_count", {
                        valueAsNumber: true,
                      })}
                      color={
                        errors.threads_archive_trigger_thread_count
                          ? "red"
                          : undefined
                      }
                    />
                    <HelperText>
                      {errors.threads_archive_trigger_thread_count && (
                        <>
                          {errors.threads_archive_trigger_thread_count.message}
                        </>
                      )}
                    </HelperText>
                  </div>
                </div>
              </AccordionContent>
            </AccordionPanel>
          </Accordion>
          <Button type="submit" className="mt-4">
            Create
          </Button>
        </form>
      </ModalBody>
    </Modal>
  );
};

export default CreateBoardModal;
