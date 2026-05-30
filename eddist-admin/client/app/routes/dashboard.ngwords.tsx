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
import { useForm } from "react-hook-form";
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import BoardMultiSelect from "~/components/BoardMultiSelect";
import CreateNgWordModal from "~/components/CreateNgWordModal";
import { getNgWords, useDeleteNgWord, useUpdateNgWord } from "~/hooks/queries";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import type { NgWord } from "~/types/entities";

const NgWords = () => {
  const { data: ngWords } = getNgWords({});
  const updateMutation = useUpdateNgWord();
  const deleteMutation = useDeleteNgWord();
  const modal = useCrudModalState<NgWord>();
  const { register, handleSubmit, control, reset } = useForm();

  return (
    <>
      <CreateNgWordModal
        open={modal.isCreateOpen}
        setOpen={(v) => {
          if (!v) modal.closeCreate();
        }}
      />
      <Modal
        show={modal.isEditOpen}
        onClose={() => {
          reset();
          modal.closeEdit();
        }}
        dismissible
      >
        <ModalHeader className="border-gray-200">Edit NG Word</ModalHeader>
        <ModalBody>
          <form
            onSubmit={handleSubmit((data) => {
              const boardIds = data.boardKeys.map((v: { value: string }) => v.value);

              updateMutation.mutate(
                {
                  params: {
                    path: {
                      ng_word_id: modal.editingItem?.id ?? "",
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
                },
              );
            })}
          >
            <div className="flex flex-col">
              <Label>Name</Label>
              <TextInput
                placeholder="Name..."
                required
                defaultValue={modal.editingItem?.name}
                {...register("name", { required: true })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Word</Label>
              <TextInput
                placeholder="Word..."
                defaultValue={modal.editingItem?.word}
                required
                {...register("word", { required: true })}
              />
            </div>
            <BoardMultiSelect
              control={control}
              name="boardKeys"
              defaultBoardIds={modal.editingItem?.boardIds ?? []}
            />
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
            type="button"
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
            {ngWords?.map((ngWord) => (
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
