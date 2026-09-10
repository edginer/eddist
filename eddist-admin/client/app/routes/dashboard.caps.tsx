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
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import CreateCapModal from "~/components/CreateCapModal";
import EditCapModal from "~/components/EditCapModal";
import { getCaps, useDeleteCap } from "~/hooks/queries";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import type { Cap } from "~/types/entities";
import { formatDateTime } from "~/utils/format";

const CapPage = () => {
  const { data: caps } = getCaps({});
  const deleteMutation = useDeleteCap();

  const modal = useCrudModalState<Cap>();

  return (
    <>
      <CreateCapModal
        open={modal.isCreateOpen}
        setOpen={(v) => {
          if (!v) modal.closeCreate();
        }}
      />

      {modal.editingItem && (
        <EditCapModal
          open={modal.isEditOpen}
          selectedCap={modal.editingItem}
          setOpen={(v) => {
            if (!v) modal.closeEdit();
          }}
        />
      )}

      <div className="p-4 sm:p-6 lg:p-8">
        <div className="flex items-center gap-3">
          <h1 className="grow text-2xl font-bold sm:text-3xl">Caps</h1>
          <button
            type="button"
            className="min-h-11 min-w-11 shrink-0 rounded-xl bg-slate-400 p-3 shadow-lg hover:bg-slate-500"
            aria-label="Create cap"
            onClick={() => modal.openCreate()}
          >
            <FaPlus />
          </button>
        </div>
        <div className="mt-4 hidden overflow-x-auto md:block">
          <Table className="min-w-[640px]">
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

        <div className="mt-4 space-y-3 md:hidden">
          {caps?.map((cap) => (
            <article
              key={cap.id}
              className="rounded-xl border border-gray-200 bg-white p-4 shadow-sm"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Cap #{cap.id}
                  </p>
                  <h2 className="mt-1 break-words font-semibold text-gray-900">{cap.name}</h2>
                </div>
                <Dropdown label={<BiDotsHorizontalRounded />}>
                  <DropdownItem onClick={() => modal.openEdit({ ...cap, boardIds: cap.board_ids })}>
                    Edit
                  </DropdownItem>
                  <DropdownItem
                    className="text-red-500"
                    onClick={() => deleteMutation.mutate({ params: { path: { cap_id: cap.id } } })}
                  >
                    Delete
                  </DropdownItem>
                </Dropdown>
              </div>
              <dl className="mt-4 grid grid-cols-1 gap-3 border-t border-gray-100 pt-4 sm:grid-cols-2">
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Created
                  </dt>
                  <dd className="mt-1 text-sm text-gray-700">{formatDateTime(cap.created_at)}</dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Updated
                  </dt>
                  <dd className="mt-1 text-sm text-gray-700">{formatDateTime(cap.updated_at)}</dd>
                </div>
              </dl>
            </article>
          ))}
          {caps?.length === 0 && (
            <div className="rounded-xl border border-dashed border-gray-300 bg-white p-8 text-center text-sm text-gray-500">
              No caps found.
            </div>
          )}
        </div>
      </div>
    </>
  );
};

export default CapPage;
