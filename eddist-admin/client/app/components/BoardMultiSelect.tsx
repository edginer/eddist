import { useMemo } from "react";
import {
  type Control,
  Controller,
  type FieldValues,
  type Path,
  type PathValue,
} from "react-hook-form";
import Select from "react-select";
import { getBoards } from "~/hooks/queries";

interface BoardOption {
  label: string;
  value: string;
}

interface BoardMultiSelectProps<TFieldValues extends FieldValues> {
  control: Control<TFieldValues>;
  name: Path<TFieldValues>;
  defaultBoardIds: string[];
}

function BoardMultiSelect<TFieldValues extends FieldValues>({
  control,
  name,
  defaultBoardIds,
}: BoardMultiSelectProps<TFieldValues>) {
  const { data: boards } = getBoards({});

  const boardOptions = useMemo<BoardOption[]>(
    () => boards?.map((b) => ({ label: b.board_key, value: b.id })) ?? [],
    [boards],
  );

  const defaultValue = useMemo<BoardOption[]>(
    () =>
      defaultBoardIds
        .map((id) => {
          const board = boards?.find((b) => b.id === id);
          return board ? { label: board.board_key, value: board.id } : null;
        })
        .filter((v): v is BoardOption => v !== null),
    [defaultBoardIds, boards],
  );

  return (
    <div className="mt-4">
      <span>Boards</span>
      <Controller
        name={name}
        control={control}
        defaultValue={defaultValue as PathValue<TFieldValues, Path<TFieldValues>>}
        render={({ field }) => (
          <Select
            options={boardOptions}
            value={boardOptions.filter((opt) =>
              field.value?.some((v: BoardOption) => v.value === opt.value),
            )}
            onChange={(value) => field.onChange(value)}
            isMulti
          />
        )}
      />
    </div>
  );
}

export default BoardMultiSelect;
