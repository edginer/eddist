import {
  Dropdown,
  DropdownItem,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from "flowbite-react";
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
          <h1 className="text-3xl font-bold grow">Caps</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => setOpenCreateCapModal(true)}
          >
            <FaPlus />
          </button>
        </div>
        <Table className="mt-4">
          <TableHead>
            <TableHeadCell>Id</TableHeadCell>
            <TableHeadCell>Cap</TableHeadCell>
            <TableHeadCell>Created At</TableHeadCell>
            <TableHeadCell>Updated At</TableHeadCell>
            <TableHeadCell></TableHeadCell>
          </TableHead>
          <TableBody className="divide-y">
            {caps?.map((cap) => (
              <TableRow key={cap.id}>
                <TableCell>{cap.id}</TableCell>
                <TableCell>{cap.name}</TableCell>
                <TableCell>{cap.created_at}</TableCell>
                <TableCell>{cap.updated_at}</TableCell>
                <TableCell>
                  <div className="text-right">
                    <Dropdown label={<BiDotsHorizontalRounded />}>
                      <DropdownItem
                        onClick={() => {
                          setOpenEditCapModal(true);
                          setSelectedCap({
                            ...cap,
                            boardIds: cap.board_ids,
                          });
                        }}
                      >
                        Edit
                      </DropdownItem>
                      <DropdownItem
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

export default CapPage;
