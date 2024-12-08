import { Dropdown, Table } from "flowbite-react";
import { useState } from "react";
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import { toast } from "react-toastify";
import CreateCapModal from "~/components/CreateCapModal";
import EditCapModal from "~/components/EditCapModal";
import { deleteCap, getCaps } from "~/hooks/queries";

export interface Cap {
  id: string;
  name: string;
  description: string;
  password?: string;
  boardIds: string[];
}

const CapPage = () => {
  const { data: caps, refetch } = getCaps({});

  const [openCreateCapModal, setOpenCreateCapModal] = useState(false);
  const [openEditCapModal, setOpenEditCapModal] = useState(false);
  const [selectedCap, setSelectedCap] = useState<Cap | undefined>(undefined);

  return (
    <>
      <CreateCapModal
        open={openCreateCapModal}
        setOpen={setOpenCreateCapModal}
        refetch={refetch}
      />

      {selectedCap && (
        <EditCapModal
          open={openEditCapModal}
          selectedCap={selectedCap}
          setOpen={setOpenEditCapModal}
          refetch={refetch}
        />
      )}

      <div className="p-2 lg:p-8">
        <div className="flex">
          <h1 className="text-3xl font-bold flex-grow">Caps</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => setOpenCreateCapModal(true)}
          >
            <FaPlus />
          </button>
        </div>
        <Table className="mt-4">
          <Table.Head>
            <Table.HeadCell>Id</Table.HeadCell>
            <Table.HeadCell>Cap</Table.HeadCell>
            <Table.HeadCell>Created At</Table.HeadCell>
            <Table.HeadCell>Updated At</Table.HeadCell>
            <Table.HeadCell></Table.HeadCell>
          </Table.Head>
          <Table.Body className="divide-y">
            {caps?.map((cap) => (
              <Table.Row key={cap.id}>
                <Table.Cell>{cap.id}</Table.Cell>
                <Table.Cell>{cap.name}</Table.Cell>
                <Table.Cell>{cap.created_at}</Table.Cell>
                <Table.Cell>{cap.updated_at}</Table.Cell>
                <Table.Cell>
                  <div className="text-right">
                    <Dropdown label={<BiDotsHorizontalRounded />}>
                      <Dropdown.Item
                        onClick={() => {
                          setOpenEditCapModal(true);
                          setSelectedCap({
                            ...cap,
                            boardIds: cap.board_ids,
                          });
                        }}
                      >
                        Edit
                      </Dropdown.Item>
                      <Dropdown.Item
                        className="text-red-500"
                        onClick={async () => {
                          const { mutate } = deleteCap({
                            params: {
                              path: {
                                cap_id: cap.id,
                              },
                            },
                          });

                          try {
                            await mutate();
                            toast.success("Successfully deleted Cap");
                            await refetch();
                          } catch (error) {
                            toast.error("Failed to delete Cap");
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

export default CapPage;
