import {
  Badge,
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

const NoticeActions = ({
  notice,
  onEdit,
  onDelete,
}: {
  notice: Notice;
  onEdit: () => void;
  onDelete: () => void;
}) => (
  <div className="flex shrink-0 gap-2">
    <Button
      size="xs"
      className="min-h-10 min-w-10 justify-center p-2"
      onClick={onEdit}
      aria-label={`Edit ${notice.title}`}
    >
      <FaEdit aria-hidden="true" />
      <span className="sr-only">Edit</span>
    </Button>
    <Button
      size="xs"
      color="alternative"
      className="min-h-10 min-w-10 justify-center p-2"
      onClick={onDelete}
      aria-label={`Delete ${notice.title}`}
    >
      <FaTrash aria-hidden="true" />
      <span className="sr-only">Delete</span>
    </Button>
  </div>
);

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
      <div className="mx-auto w-full max-w-7xl p-4 sm:p-6 lg:p-8">
        <div className="mb-5 flex flex-col gap-3 border-b border-gray-200 pb-5 sm:flex-row sm:items-center sm:justify-between">
          <h1 className="text-2xl font-bold text-gray-900 sm:text-3xl">Notices</h1>
          <Button className="w-full sm:w-auto" onClick={() => modal.openCreate()}>
            <FaPlus className="mr-2" />
            Create Notice
          </Button>
        </div>

        <div className="hidden overflow-x-auto rounded-xl border border-gray-200 bg-white md:block">
          <Table hoverable className="min-w-[700px]">
            <TableHead>
              <TableHeadCell>Title</TableHeadCell>
              <TableHeadCell>Slug</TableHeadCell>
              <TableHeadCell>Published At</TableHeadCell>
              <TableHeadCell>Visibility</TableHeadCell>
              <TableHeadCell>Actions</TableHeadCell>
            </TableHead>
            <TableBody>
              {notices?.map((notice) => (
                <TableRow className="border-gray-200" key={notice.id}>
                  <TableCell className="font-medium">{notice.title}</TableCell>
                  <TableCell>
                    <code className="text-sm text-gray-600">{notice.slug}</code>
                  </TableCell>
                  <TableCell>{formatDateTime(notice.published_at)}</TableCell>
                  <TableCell>
                    {notice.hide_from_list ? (
                      <Badge color="warning">Hidden from list</Badge>
                    ) : (
                      <Badge color="success">Listed</Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <NoticeActions
                      notice={notice}
                      onEdit={() => modal.openEdit(notice)}
                      onDelete={() => handleDelete(notice.id)}
                    />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="space-y-3 md:hidden">
          {notices?.map((notice) => (
            <article
              key={notice.id}
              className="rounded-xl border border-gray-200 bg-white p-4 shadow-sm"
            >
              <div className="flex items-start justify-between gap-3">
                <h2 className="min-w-0 break-words font-semibold text-gray-900">{notice.title}</h2>
                <NoticeActions
                  notice={notice}
                  onEdit={() => modal.openEdit(notice)}
                  onDelete={() => handleDelete(notice.id)}
                />
              </div>
              <dl className="mt-4 space-y-3 border-t border-gray-100 pt-4">
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Slug
                  </dt>
                  <dd className="mt-1 break-all font-mono text-sm text-gray-700">{notice.slug}</dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Published at
                  </dt>
                  <dd className="mt-1 text-sm text-gray-700">
                    {formatDateTime(notice.published_at)}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Visibility
                  </dt>
                  <dd className="mt-1">
                    {notice.hide_from_list ? (
                      <Badge color="warning">Hidden from list</Badge>
                    ) : (
                      <Badge color="success">Listed</Badge>
                    )}
                  </dd>
                </div>
              </dl>
            </article>
          ))}
          {notices?.length === 0 && (
            <div className="rounded-xl border border-dashed border-gray-300 bg-white p-8 text-center text-sm text-gray-500">
              No notices found.
            </div>
          )}
        </div>
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
