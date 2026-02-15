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
import { useCrudModalState } from "~/hooks/useCrudModalState";
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import CreateCapModal from "~/components/CreateCapModal";
import EditCapModal from "~/components/EditCapModal";
import { getCaps, useDeleteCap } from "~/hooks/queries";
import { formatDateTime } from "~/utils/format";

export interface Cap {
  id: string;
  name: string;
  description: string;
  password?: string;
  boardIds: string[];
}

const CapPage = () => {
  const { data: caps, refetch } = getCaps({});
  const deleteMutation = useDeleteCap();

  const modal = useCrudModalState<Cap>();

  return (
    <>
      <CreateCapModal
        open={modal.isCreateOpen}
        setOpen={(v) => { if (!v) modal.closeCreate(); }}
        refetch={refetch}
      />

      {modal.editingItem && (
        <EditCapModal
          open={modal.isEditOpen}
          selectedCap={modal.editingItem}
          setOpen={(v) => { if (!v) modal.closeEdit(); }}
          refetch={refetch}
        />
      )}

      <div className="p-2 lg:p-8">
        <div className="flex">
          <h1 className="text-3xl font-bold grow">Caps</h1>
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
            <TableHeadCell>Cap</TableHeadCell>
            <TableHeadCell>Created At</TableHeadCell>
            <TableHeadCell>Updated At</TableHeadCell>
            <TableHeadCell></TableHeadCell>
          </TableHead>
          <TableBody className="divide-y">
            {caps?.map((cap) => (
              <TableRow className="border-gray-200" key={cap.id}>
                <TableCell>{cap.id}</TableCell>
                <TableCell>{cap.name}</TableCell>
                <TableCell>{formatDateTime(cap.created_at)}</TableCell>
                <TableCell>{formatDateTime(cap.updated_at)}</TableCell>
                <TableCell>
                  <div className="text-right">
                    <Dropdown label={<BiDotsHorizontalRounded />}>
                      <DropdownItem
                        onClick={() => {
                          modal.openEdit({
                            ...cap,
                            boardIds: cap.board_ids,
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
                                cap_id: cap.id,
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

export default CapPage;
