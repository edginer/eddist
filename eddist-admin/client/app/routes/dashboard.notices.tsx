import {
  Button,
  Modal,
  ModalBody,
  ModalHeader,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from "flowbite-react";
import { FaEdit, FaPlus, FaTrash } from "react-icons/fa";
import NoticeForm from "~/components/NoticeForm";
import { getNotices, useCreateNotice, useDeleteNotice, useUpdateNotice } from "~/hooks/queries";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import type { paths } from "~/openapi/schema";
import { formatDateTime } from "~/utils/format";

type Notice = paths["/notices/"]["get"]["responses"]["200"]["content"]["application/json"][number];

const Notices = () => {
  const modal = useCrudModalState<Notice>();

  const createMutation = useCreateNotice();
  const updateMutation = useUpdateNotice();
  const deleteMutation = useDeleteNotice();

  const { data: notices } = getNotices({});

  const handleDelete = (id: string) => {
    if (window.confirm("Are you sure you want to delete this notice?")) {
      deleteMutation.mutate({ params: { path: { id } } });
    }
  };

  return (
    <>
      <div className="p-4">
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-2xl font-bold">Notices</h1>
          <Button onClick={() => modal.openCreate()}>
            <FaPlus className="mr-2" />
            Create Notice
          </Button>
        </div>

        <Table>
          <TableHead>
            <TableHeadCell>Title</TableHeadCell>
            <TableHeadCell>Slug</TableHeadCell>
            <TableHeadCell>Published At</TableHeadCell>
            <TableHeadCell>Actions</TableHeadCell>
          </TableHead>
          <TableBody>
            {notices?.map((notice) => (
              <TableRow className="border-gray-200" key={notice.id}>
                <TableCell>{notice.title}</TableCell>
                <TableCell>
                  <code className="text-sm text-gray-600">{notice.slug}</code>
                </TableCell>
                <TableCell>{formatDateTime(notice.published_at)}</TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button size="xs" onClick={() => modal.openEdit(notice)}>
                      <FaEdit />
                    </Button>
                    <Button size="xs" color="alternative" onClick={() => handleDelete(notice.id)}>
                      <FaTrash />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* Create Modal */}
      <Modal show={modal.isCreateOpen} onClose={() => modal.closeCreate()} dismissible>
        <ModalHeader className="border-gray-200">Create Notice</ModalHeader>
        <ModalBody>
          <NoticeForm
            mode="create"
            onSubmit={(data) => {
              createMutation.mutate({ body: data }, { onSuccess: () => modal.closeCreate() });
            }}
          />
        </ModalBody>
      </Modal>

      {/* Edit Modal */}
      {modal.editingItem && (
        <Modal show={modal.isEditOpen} onClose={() => modal.closeEdit()} dismissible>
          <ModalHeader className="border-gray-200">Edit Notice</ModalHeader>
          <ModalBody>
            <NoticeForm
              mode="edit"
              defaultValues={modal.editingItem}
              onSubmit={(data) => {
                updateMutation.mutate(
                  {
                    params: { path: { id: modal.editingItem?.id ?? "" } },
                    body: data,
                  },
                  { onSuccess: () => modal.closeEdit() },
                );
              }}
            />
          </ModalBody>
        </Modal>
      )}
    </>
  );
};

export default Notices;
