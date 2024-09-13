import {
  Button,
  Checkbox,
  Dropdown,
  Label,
  Modal,
  Table,
  TextInput,
} from "flowbite-react";
import { useState } from "react";
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import { useFieldArray, useForm } from "react-hook-form";
import {
  deleteNgWord,
  getBoards,
  getNgWords,
  updateNgWord,
} from "~/hooks/queries";
import CreateNgWordModal from "~/components/CreateNgWordModal";
import { toast } from "react-toastify";

interface NgWord {
  id: string;
  name: string;
  word: string;
  boardIds: string[];
}

const NgWords = () => {
  const { data: ngWords, refetch } = getNgWords({});
  const [openCreateNgModal, setOpenCreateNgModal] = useState(false);
  const [openEditNgModal, setOpenEditNgModal] = useState(false);
  const [selectedNgWord, setSelectedNgWord] = useState<NgWord | undefined>(
    undefined
  );
  const { register, handleSubmit, control, getValues } = useForm();
  const { fields } = useFieldArray({
    control,
    name: "boardIds",
  });
  const { data: boards } = getBoards({});

  return (
    <>
      <CreateNgWordModal
        open={openCreateNgModal}
        setOpen={setOpenCreateNgModal}
        refetch={refetch}
      />
      <Modal show={openEditNgModal} onClose={() => setOpenEditNgModal(false)}>
        <Modal.Header>Edit NG Word</Modal.Header>
        <Modal.Body>
          <form
            onSubmit={handleSubmit(async (data) => {
              try {
                const boardIds = Object.entries(data.boardIds)
                  .map((val: [string, unknown]) => {
                    const boardKey = val[0];
                    const boardId = boards!.find(
                      (board) => board.board_key === boardKey
                    )?.id;
                    return boardId;
                  })
                  .filter((x) => x != null);
                const { mutate } = updateNgWord({
                  params: {
                    path: {
                      ng_word_id: selectedNgWord!.id,
                    },
                  },
                  body: {
                    name: data.name,
                    word: data.word,
                    board_ids: boardIds,
                  },
                });
                await mutate();
                setOpenEditNgModal(false);
                toast.success("Successfully updated NG word");
                await refetch();
              } catch (e) {
                toast.error("Failed to update NG word");
              }
            })}
          >
            <div className="flex flex-col">
              <input
                type="hidden"
                {...(register("id"),
                {
                  value: selectedNgWord?.id,
                })}
              />
              <Label>Name</Label>
              <TextInput
                placeholder="Name..."
                required
                defaultValue={selectedNgWord?.name}
                {...register("name", {
                  required: true,
                })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Word</Label>
              <TextInput
                placeholder="Word..."
                defaultValue={selectedNgWord?.word}
                required
                {...register("word", {
                  required: true,
                })}
              />
            </div>
            <div>
              Boards
              <div>
                {boards!.map((board, index) => (
                  <div key={`board-key:${board.board_key}`}>
                    <Checkbox
                      id={`board-key-checkbox-${index}`}
                      key={`board-key:${board.board_key}`}
                      {...register(`boardIds.${board.board_key}`)}
                      defaultChecked={selectedNgWord?.boardIds.includes(
                        board.board_key
                      )}
                    />
                    <Label
                      htmlFor={`board-key-checkbox-${board.board_key}`}
                      className="ml-2"
                    >
                      {board.name}
                    </Label>
                  </div>
                ))}
              </div>
            </div>
            <Button type="submit" className="mt-4">
              Create
            </Button>
          </form>
        </Modal.Body>
      </Modal>
      <div className="p-2 lg:p-8">
        <div className="flex">
          <h1 className="text-3xl font-bold flex-grow">NG words</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => setOpenCreateNgModal(true)}
          >
            <FaPlus />
          </button>
        </div>
        <Table className="mt-4">
          <Table.Head>
            <Table.HeadCell>Id</Table.HeadCell>
            <Table.HeadCell>Name</Table.HeadCell>
            <Table.HeadCell>Word</Table.HeadCell>
            <Table.HeadCell></Table.HeadCell>
          </Table.Head>
          <Table.Body className="divide-y">
            {ngWords!.map((ngWord) => (
              <Table.Row key={ngWord.id}>
                <Table.Cell>{ngWord.id}</Table.Cell>
                <Table.Cell>{ngWord.name}</Table.Cell>
                <Table.Cell>{ngWord.word}</Table.Cell>
                <Table.Cell>
                  <div className="text-right">
                    <Dropdown label={<BiDotsHorizontalRounded />}>
                      <Dropdown.Item
                        onClick={() => {
                          setOpenEditNgModal(true);
                          setSelectedNgWord({
                            ...ngWord,
                            boardIds: ngWord.board_ids,
                          });
                        }}
                      >
                        Edit
                      </Dropdown.Item>
                      <Dropdown.Item
                        className="text-red-500"
                        onClick={async () => {
                          try {
                            const { mutate } = deleteNgWord({
                              params: {
                                path: {
                                  ng_word_id: ngWord.id,
                                },
                              },
                            });
                            await mutate();
                            toast.success("Successfully deleted NG word");
                            await refetch();
                          } catch (e) {
                            toast.error("Failed to delete NG word");
                          }
                        }}
                      >
                        Delete
                      </Dropdown.Item>
                    </Dropdown>
                  </div>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      </div>
    </>
  );
};

export default NgWords;
