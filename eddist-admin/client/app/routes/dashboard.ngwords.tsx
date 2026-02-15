import {
  Button,
  Dropdown,
  DropdownItem,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
  TextInput,
} from "flowbite-react";
import { useMemo } from "react";
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import { Controller, useForm } from "react-hook-form";
import {
  getBoards,
  getNgWords,
  useUpdateNgWord,
  useDeleteNgWord,
} from "~/hooks/queries";
import CreateNgWordModal from "~/components/CreateNgWordModal";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import Select from "react-select";

interface NgWord {
  id: string;
  name: string;
  word: string;
  boardIds: string[];
}

interface BoardSelectOption {
  label: string;
  value: string;
}

const NgWords = () => {
  const { data: ngWords, refetch } = getNgWords({});
  const updateMutation = useUpdateNgWord();
  const deleteMutation = useDeleteNgWord();
  const modal = useCrudModalState<NgWord>();
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
    <>
      <CreateNgWordModal
        open={modal.isCreateOpen}
        setOpen={(v) => { if (!v) modal.closeCreate(); }}
        refetch={refetch}
      />
      <Modal
        show={modal.isEditOpen}
        onClose={() => {
          reset();
          modal.closeEdit();
        }}
      >
        <ModalHeader className="border-gray-200">Edit NG Word</ModalHeader>
        <ModalBody>
          <form
            onSubmit={handleSubmit((data) => {
              console.log(data);
              const boardIds = data.boardKeys.map(
                (val: BoardSelectOption) =>
                  boards!.find((board) => board.board_key === val.value)?.id
              );
              console.log(boardIds);

              updateMutation.mutate(
                {
                  params: {
                    path: {
                      ng_word_id: modal.editingItem!.id,
                    },
                  },
                  body: {
                    name: data.name,
                    word: data.word,
                    board_ids: boardIds,
                  },
                },
                {
                  onSuccess: () => {
                    modal.closeEdit();
                    reset();
                  },
                }
              );
            })}
          >
            <div className="flex flex-col">
              <input
                type="hidden"
                {...(register("id"),
                {
                  value: modal.editingItem?.id,
                })}
              />
              <Label>Name</Label>
              <TextInput
                placeholder="Name..."
                required
                defaultValue={modal.editingItem?.name}
                {...register("name", {
                  required: true,
                })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Word</Label>
              <TextInput
                placeholder="Word..."
                defaultValue={modal.editingItem?.word}
                required
                {...register("word", {
                  required: true,
                })}
              />
            </div>
            <div>
              Boards
              <Controller
                name="boardKeys"
                control={control}
                defaultValue={modal.editingItem?.boardIds.map((boardId) => {
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
            <Button type="submit" className="mt-4">
              Submit
            </Button>
          </form>
        </ModalBody>
      </Modal>
      <div className="p-2 lg:p-8">
        <div className="flex">
          <h1 className="text-3xl font-bold grow">NG words</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => modal.openCreate()}
          >
            <FaPlus />
          </button>
        </div>
        <Table className="mt-4">
          <TableHead>
            <TableHeadCell>Id</TableHeadCell>
            <TableHeadCell>Name</TableHeadCell>
            <TableHeadCell>Word</TableHeadCell>
            <TableHeadCell></TableHeadCell>
          </TableHead>
          <TableBody className="divide-y">
            {ngWords!.map((ngWord) => (
              <TableRow className="border-gray-200" key={ngWord.id}>
                <TableCell>{ngWord.id}</TableCell>
                <TableCell>{ngWord.name}</TableCell>
                <TableCell>{ngWord.word}</TableCell>
                <TableCell>
                  <div className="text-right">
                    <Dropdown label={<BiDotsHorizontalRounded />}>
                      <DropdownItem
                        onClick={() => {
                          modal.openEdit({
                            ...ngWord,
                            boardIds: ngWord.board_ids,
                          });
                        }}
                      >
                        Edit
                      </DropdownItem>
                      <DropdownItem
                        className="text-red-500"
                        onClick={() => {
                          deleteMutation.mutate({
                            params: {
                              path: {
                                ng_word_id: ngWord.id,
                              },
                            },
                          });
                        }}
                      >
                        Delete
                      </DropdownItem>
                    </Dropdown>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </>
  );
};

export default NgWords;
