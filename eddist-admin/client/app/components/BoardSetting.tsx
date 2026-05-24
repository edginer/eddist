import { zodResolver } from "@hookform/resolvers/zod";
import { Button, Checkbox, HelperText, Label, Select, Textarea, TextInput } from "flowbite-react";
import type React from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { getBoardInfo, useUpdateBoard } from "~/hooks/queries";
import type { components } from "~/openapi/schema";

type Board = Omit<components["schemas"]["Board"], "thread_count">;
type BoardInfo = components["schemas"]["BoardInfo"];

const boardGeneralSettingSchema = z.object({
  name: z.string().min(1).max(64),
  default_name: z.string().min(1).max(64),
  local_rule: z.string().min(1),
  read_only: z.boolean(),
  force_metadent_type: z.string().optional().nullable(),
});

const boardPostRestrictionSettingSchema = z.object({
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
});

const boardThreadsArchiveSettingSchema = z.object({
  threads_archive_cron: z.string().optional(),
  threads_archive_trigger_thread_count: z
    .union([z.number().positive(), z.nan().transform(() => undefined)])
    .optional(),
});

const boardThreadStopperSettingSchema = z.object({
  enable_1001_message: z.boolean(),
  custom_1001_message: z.string().optional().nullable(),
});

const SettingField: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return <div className="flex flex-col mb-2">{children}</div>;
};

const GeneralSetting: React.FC<{
  board: Board;
  boardInfo: BoardInfo;
  refetch: () => Promise<void>;
}> = ({ board, boardInfo, refetch }) => {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<z.infer<typeof boardGeneralSettingSchema>>({
    resolver: zodResolver(boardGeneralSettingSchema),
  });

  const updateBoardMutation = useUpdateBoard();

  return (
    <form
      onSubmit={handleSubmit((data) => {
        updateBoardMutation.mutate(
          {
            params: {
              path: {
                board_key: board.board_key,
              },
            },
            body: data,
          },
          {
            onSuccess: () => refetch(),
          },
        );
      })}
    >
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">General Setting</h2>
      <SettingField>
        <Label>Board Key</Label>
        <TextInput value={board.board_key} disabled className="mt-1" />
      </SettingField>
      <SettingField>
        <Label>Board Name</Label>
        <TextInput
          {...register("name")}
          defaultValue={board?.name}
          color={errors.name ? "red" : undefined}
          className="mt-1"
        />
        <HelperText>{errors.name?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Default Name</Label>
        <TextInput
          {...register("default_name")}
          defaultValue={board?.default_name}
          color={errors.default_name ? "red" : undefined}
          className="mt-1"
        />
        <HelperText>{errors.default_name?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Read Only</Label>
        <Checkbox
          className="mt-1"
          {...register("read_only")}
          defaultChecked={boardInfo?.read_only}
        />
      </SettingField>
      <SettingField>
        <Label>Force Metadent Type</Label>
        <Select
          className="mt-1"
          {...register("force_metadent_type")}
          defaultValue={boardInfo?.force_metadent_type ?? ""}
        >
          <option value="">None (User opt-in)</option>
          <option value="v">Verbose (Level only)</option>
          <option value="vv">VVerbose (ID string)</option>
          <option value="vvv">VVVerbose (Level + ID)</option>
        </Select>
        <HelperText>Force all posts to use this metadent type</HelperText>
      </SettingField>
      <SettingField>
        <Label>Local Rule</Label>
        <Textarea
          {...register("local_rule")}
          defaultValue={boardInfo?.local_rules}
          className="mt-1"
          color={errors.local_rule ? "red" : undefined}
          rows={7}
        />
        <HelperText>{errors.local_rule?.message}</HelperText>
      </SettingField>
      <Button type="submit">Update</Button>
    </form>
  );
};

const PostRestrictionSetting: React.FC<{
  board: Board;
  boardInfo: BoardInfo;
  refetch: () => Promise<void>;
}> = ({ board, boardInfo, refetch }) => {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<z.infer<typeof boardPostRestrictionSettingSchema>>({
    resolver: zodResolver(boardPostRestrictionSettingSchema),
  });

  const updateBoardMutation = useUpdateBoard();

  return (
    <form
      onSubmit={handleSubmit((data) => {
        updateBoardMutation.mutate(
          {
            params: {
              path: {
                board_key: board.board_key,
              },
            },
            body: data,
          },
          {
            onSuccess: () => refetch(),
          },
        );
      })}
    >
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">Post Restriction Setting</h2>
      <SettingField>
        <Label>Base Thread Creation Span (sec)</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("base_thread_creation_span_sec", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo?.base_thread_creation_span_sec}
          color={errors.base_thread_creation_span_sec ? "red" : undefined}
        />
        <HelperText>{errors.base_thread_creation_span_sec?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Base Response Creation Span (sec)</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("base_response_creation_span_sec", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo?.base_response_creation_span_sec}
          color={errors.base_response_creation_span_sec ? "red" : undefined}
        />
        <HelperText>{errors.base_response_creation_span_sec?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Thread Name Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_thread_name_byte_length", { valueAsNumber: true })}
          defaultValue={boardInfo?.max_thread_name_byte_length}
          color={errors.max_thread_name_byte_length ? "red" : undefined}
        />
        <HelperText>{errors.max_thread_name_byte_length?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Author Name Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_author_name_byte_length", { valueAsNumber: true })}
          defaultValue={boardInfo?.max_author_name_byte_length}
          color={errors.max_author_name_byte_length ? "red" : undefined}
        />
        <HelperText>{errors.max_author_name_byte_length?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Email Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_email_byte_length", { valueAsNumber: true })}
          defaultValue={boardInfo?.max_email_byte_length}
          color={errors.max_email_byte_length ? "red" : undefined}
        />
        <HelperText>{errors.max_email_byte_length?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Response Body Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_response_body_byte_length", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo?.max_response_body_byte_length}
          color={errors.max_response_body_byte_length ? "red" : undefined}
        />
        <HelperText>{errors.max_response_body_byte_length?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Response Body Lines</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_response_body_lines", { valueAsNumber: true })}
          defaultValue={boardInfo?.max_response_body_lines}
          color={errors.max_response_body_lines ? "red" : undefined}
        />
        <HelperText>{errors.max_response_body_lines?.message}</HelperText>
      </SettingField>
      <Button type="submit">Update</Button>
    </form>
  );
};

const ThreadsArchiveSetting: React.FC<{
  board: Board;
  boardInfo: BoardInfo;
  refetch: () => Promise<void>;
}> = ({ board, boardInfo, refetch }) => {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<z.infer<typeof boardThreadsArchiveSettingSchema>>({
    resolver: zodResolver(boardThreadsArchiveSettingSchema),
  });

  const updateBoardMutation = useUpdateBoard();

  return (
    <form
      onSubmit={handleSubmit((data) => {
        updateBoardMutation.mutate(
          {
            params: {
              path: {
                board_key: board.board_key,
              },
            },
            body: data,
          },
          {
            onSuccess: () => refetch(),
          },
        );
      })}
    >
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">Threads Archive Setting</h2>
      <SettingField>
        <Label>Threads Archive Cron String</Label>
        <TextInput
          className="mt-1"
          {...register("threads_archive_cron")}
          defaultValue={boardInfo?.threads_archive_cron || ""}
          color={errors.threads_archive_cron ? "red" : undefined}
        />
        <HelperText>{errors.threads_archive_cron?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Threads Archive Trigger Thread Count</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("threads_archive_trigger_thread_count", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo?.threads_archive_trigger_thread_count || ""}
          color={errors.threads_archive_trigger_thread_count ? "red" : undefined}
        />
        <HelperText>{errors.threads_archive_trigger_thread_count?.message}</HelperText>
      </SettingField>
      <Button type="submit">Update</Button>
    </form>
  );
};

const ThreadStopperSetting: React.FC<{
  board: Board;
  boardInfo: BoardInfo;
  refetch: () => Promise<void>;
}> = ({ board, boardInfo, refetch }) => {
  const { register, handleSubmit, watch } = useForm<
    z.infer<typeof boardThreadStopperSettingSchema>
  >({
    resolver: zodResolver(boardThreadStopperSettingSchema),
    defaultValues: {
      enable_1001_message: boardInfo.enable_1001_message,
      custom_1001_message: boardInfo.custom_1001_message ?? "",
    },
  });

  const updateBoardMutation = useUpdateBoard();
  const enableChecked = watch("enable_1001_message");

  return (
    <form
      onSubmit={handleSubmit((data) => {
        updateBoardMutation.mutate(
          {
            params: { path: { board_key: board.board_key } },
            body: {
              enable_1001_message: data.enable_1001_message,
              custom_1001_message: data.custom_1001_message || "",
            },
          },
          { onSuccess: () => refetch() },
        );
      })}
    >
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">Thread Stopper Setting</h2>
      <SettingField>
        <Label>Enable 1001 Message</Label>
        <Checkbox className="mt-1" {...register("enable_1001_message")} />
        <HelperText>Show the 1001 stopper message at the bottom of full threads</HelperText>
      </SettingField>
      <SettingField>
        <Label>Custom Message Body</Label>
        <Textarea
          {...register("custom_1001_message")}
          disabled={!enableChecked}
          className="mt-1"
          rows={4}
          placeholder="Leave empty to use the default Japanese message with life time"
        />
        <HelperText>
          Replaces the default body text. Leave blank for the default message.
        </HelperText>
      </SettingField>
      <Button type="submit">Update</Button>
    </form>
  );
};

const BoardSetting = ({
  board,
  refetchBoard,
}: {
  board: Board;
  refetchBoard: () => Promise<unknown>;
}) => {
  const { data: boardInfo, refetch: refetchBoardInfo } = getBoardInfo({
    params: {
      path: {
        board_key: board.board_key,
      },
    },
  });

  const refetch = async () => {
    await refetchBoard();
    await refetchBoardInfo();
  };

  if (!boardInfo) return null;

  return (
    <div>
      <GeneralSetting board={board} boardInfo={boardInfo} refetch={refetch} />
      <hr className="bg-gray-500 border-0 h-px my-4" />
      <PostRestrictionSetting board={board} boardInfo={boardInfo} refetch={refetch} />
      <hr className="bg-gray-500 border-0 h-px my-4" />
      <ThreadsArchiveSetting board={board} boardInfo={boardInfo} refetch={refetch} />
      <hr className="bg-gray-500 border-0 h-px my-4" />
      <ThreadStopperSetting board={board} boardInfo={boardInfo} refetch={refetch} />
    </div>
  );
};

export default BoardSetting;
