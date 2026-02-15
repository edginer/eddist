import { zodResolver } from "@hookform/resolvers/zod";
import {
  Button,
  Checkbox,
  HelperText,
  Label,
  Textarea,
  TextInput,
} from "flowbite-react";
import React from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { getBoardInfo, useUpdateBoard } from "~/hooks/queries";
import { components } from "~/openapi/schema";

type Board = Omit<components["schemas"]["Board"], "thread_count">;
type BoardInfo = components["schemas"]["BoardInfo"];

const boardGeneralSettingSchema = z.object({
  name: z.string().min(1).max(64),
  default_name: z.string().min(1).max(64),
  local_rule: z.string().min(1),
  read_only: z.boolean(),
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

const SettingField: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
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
  } = useForm<components["schemas"]["EditBoardInput"]>({
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
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">
        General Setting
      </h2>
      <SettingField>
        <Label>Board Key</Label>
        <TextInput value={board.board_key} disabled className="mt-1" />
      </SettingField>
      <SettingField>
        <Label>Board Name</Label>
        <TextInput
          {...register("name")}
          defaultValue={board!.name}
          color={errors.name ? "red" : undefined}
          className="mt-1"
        />
        <HelperText>{errors.name && errors.name?.message}</HelperText>
      </SettingField>
      <SettingField>
        <Label>Default Name</Label>
        <TextInput
          {...register("default_name")}
          defaultValue={board!.default_name}
          color={errors.default_name ? "red" : undefined}
          className="mt-1"
        />
        <HelperText>
          {errors.default_name && errors.default_name.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Read Only</Label>
        <Checkbox
          className="mt-1"
          {...register("read_only")}
          defaultChecked={boardInfo!.read_only}
        />
      </SettingField>
      <SettingField>
        <Label>Local Rule</Label>
        <Textarea
          {...register("local_rule")}
          defaultValue={boardInfo!.local_rules}
          className="mt-1"
          color={errors.local_rule ? "red" : undefined}
          rows={7}
        />
        <HelperText>
          {errors.local_rule && errors.local_rule.message}
        </HelperText>
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
  } = useForm<components["schemas"]["EditBoardInput"]>({
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
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">
        Post Restriction Setting
      </h2>
      <SettingField>
        <Label>Base Thread Creation Span (sec)</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("base_thread_creation_span_sec", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo!.base_thread_creation_span_sec}
          color={errors.base_thread_creation_span_sec ? "red" : undefined}
        />
        <HelperText>
          {errors.base_thread_creation_span_sec &&
            errors.base_thread_creation_span_sec.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Base Response Creation Span (sec)</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("base_response_creation_span_sec", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo!.base_response_creation_span_sec}
          color={errors.base_response_creation_span_sec ? "red" : undefined}
        />
        <HelperText>
          {errors.base_response_creation_span_sec &&
            errors.base_response_creation_span_sec.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Thread Name Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_thread_name_byte_length", { valueAsNumber: true })}
          defaultValue={boardInfo!.max_thread_name_byte_length}
          color={errors.max_thread_name_byte_length ? "red" : undefined}
        />
        <HelperText>
          {errors.max_thread_name_byte_length &&
            errors.max_thread_name_byte_length.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Author Name Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_author_name_byte_length", { valueAsNumber: true })}
          defaultValue={boardInfo!.max_author_name_byte_length}
          color={errors.max_author_name_byte_length ? "red" : undefined}
        />
        <HelperText>
          {errors.max_author_name_byte_length &&
            errors.max_author_name_byte_length.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Email Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_email_byte_length", { valueAsNumber: true })}
          defaultValue={boardInfo!.max_email_byte_length}
          color={errors.max_email_byte_length ? "red" : undefined}
        />
        <HelperText>
          {errors.max_email_byte_length && errors.max_email_byte_length.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Response Body Byte Length</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_response_body_byte_length", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo!.max_response_body_byte_length}
          color={errors.max_response_body_byte_length ? "red" : undefined}
        />
        <HelperText>
          {errors.max_response_body_byte_length &&
            errors.max_response_body_byte_length.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Max Response Body Lines</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("max_response_body_lines", { valueAsNumber: true })}
          defaultValue={boardInfo!.max_response_body_lines}
          color={errors.max_response_body_lines ? "red" : undefined}
        />
        <HelperText>
          {errors.max_response_body_lines &&
            errors.max_response_body_lines.message}
        </HelperText>
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
  } = useForm<components["schemas"]["EditBoardInput"]>({
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
      <h2 className="text-2xl font-semibold text-gray-700 mb-4">
        Threads Archive Setting
      </h2>
      <SettingField>
        <Label>Threads Archive Cron String</Label>
        <TextInput
          className="mt-1"
          {...register("threads_archive_cron")}
          defaultValue={boardInfo!.threads_archive_cron || ""}
          color={errors.threads_archive_cron ? "red" : undefined}
        />
        <HelperText>
          {errors.threads_archive_cron && errors.threads_archive_cron.message}
        </HelperText>
      </SettingField>
      <SettingField>
        <Label>Threads Archive Trigger Thread Count</Label>
        <TextInput
          className="mt-1"
          type="number"
          {...register("threads_archive_trigger_thread_count", {
            valueAsNumber: true,
          })}
          defaultValue={boardInfo!.threads_archive_trigger_thread_count || ""}
          color={
            errors.threads_archive_trigger_thread_count ? "red" : undefined
          }
        />
        <HelperText>
          {errors.threads_archive_trigger_thread_count &&
            errors.threads_archive_trigger_thread_count.message}
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

  return (
    <div>
      <GeneralSetting board={board} boardInfo={boardInfo!} refetch={refetch} />
      <hr className="bg-gray-500 border-0 h-px my-4" />
      <PostRestrictionSetting
        board={board}
        boardInfo={boardInfo!}
        refetch={refetch}
      />
      <hr className="bg-gray-500 border-0 h-px my-4" />
      <ThreadsArchiveSetting
        board={board}
        boardInfo={boardInfo!}
        refetch={refetch}
      />
    </div>
  );
};

export default BoardSetting;
