import {
  Button,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  TextInput,
} from "flowbite-react";
import React, { useMemo } from "react";
import { Controller, useForm } from "react-hook-form";
import { getBoards, updateCap } from "~/hooks/queries";
import Select from "react-select";
import { toast } from "react-toastify";
import { Cap } from "~/routes/dashboard.caps";

interface EditCapModalProps {
  open: boolean;
  selectedCap: Cap;
  setOpen: (open: boolean) => void;
  refetch: () => Promise<unknown>;
}

interface BoardSelectOption {
  label: string;
  value: string;
}

const EditCapModal = ({
  open,
  selectedCap,
  setOpen,
  refetch,
}: EditCapModalProps) => {
  const { register, handleSubmit, control, reset } = useForm();

  const { data: boards } = getBoards({});
  const boardSelectOptions = useMemo(() => {
    if (boards) {
      return boards.map((board) => ({
        label: board.board_key,
        value: board.board_key,
      }));
    }
    return [];
  }, [boards]);

  return (
    <Modal
      show={open}
      onClose={() => {
        setOpen(false);
        reset();
      }}
    >
      <ModalHeader>Edit Cap</ModalHeader>
      <ModalBody>
        <form
          onSubmit={handleSubmit(async (data) => {
            try {
              const boardIds = data.boardKeys.map(
                (val: BoardSelectOption) =>
                  boards!.find((board) => board.board_key === val.value)?.id
              );
              const password = data.password ? data.password : undefined;

              const { mutate } = updateCap({
                params: {
                  path: {
                    cap_id: selectedCap.id,
                  },
                },
                body: {
                  name: data.name,
                  description: data.description,
                  password,
                  board_ids: boardIds,
                },
              });
              await mutate();
              setOpen(false);
              reset();
              toast.success("Successfully updated Cap");
              await refetch();
            } catch (error) {
              toast.error("Failed to update Cap");
            }
          })}
        >
          <div className="flex flex-col">
            <Label>Name</Label>
            <TextInput
              placeholder="Name..."
              required
              defaultValue={selectedCap.name}
              {...register("name", {
                required: true,
              })}
            />
          </div>
          <div className="flex flex-col mt-4">
            <Label>Description</Label>
            <TextInput
              placeholder="Description..."
              defaultValue={selectedCap.description}
              {...register("description", {
                required: false,
              })}
            />
          </div>
          <div className="flex flex-col mt-4">
            <Label>Password</Label>
            <TextInput
              placeholder="Password..."
              type="password"
              {...register("password", {
                required: false,
              })}
            />
          </div>
          <div>
            Boards
            <Controller
              name="boardKeys"
              control={control}
              defaultValue={selectedCap?.boardIds.map((boardId) => {
                const board = boards!.find((b) => b.id === boardId);
                return {
                  label: board!.board_key,
                  value: board!.board_key,
                };
              })}
              render={({ field }) => (
                <Select
                  options={boardSelectOptions}
                  value={boardSelectOptions
                    .map((board) => {
                      if (
                        field.value?.find(
                          (v: BoardSelectOption) => v.value === board.value
                        )
                      ) {
                        return board;
                      }
                      return null;
                    })
                    .filter((board) => board != null)}
                  onChange={(value) => {
                    field.onChange(value);
                  }}
                  isMulti
                />
              )}
            />
          </div>
          <div className="flex justify-end mt-4">
            <Button type="submit">Submit</Button>
          </div>
        </form>
      </ModalBody>
    </Modal>
  );
};

export default EditCapModal;
