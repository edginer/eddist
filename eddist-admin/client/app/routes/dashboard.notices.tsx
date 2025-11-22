import {
  Button,
  Label,
  Modal,
  Table,
  Textarea,
  TextInput,
} from "flowbite-react";
import { useState } from "react";
import { FaPlus, FaEdit, FaTrash, FaSync } from "react-icons/fa";
import { useForm } from "react-hook-form";
import { toast } from "react-toastify";
import { useQueryClient } from "@tanstack/react-query";
import {
  getNotices,
  createNotice,
  updateNotice,
  deleteNotice,
} from "~/hooks/queries";
import type { paths } from "~/openapi/schema";

// Generate URL-safe slug from title
function generateSlug(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, "-")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

type Notice =
  paths["/notices/"]["get"]["responses"]["200"]["content"]["application/json"][number];
type NoticeFormData =
  paths["/notices/"]["post"]["requestBody"]["content"]["application/json"];

const Notices = () => {
  const queryClient = useQueryClient();
  const [openCreateModal, setOpenCreateModal] = useState(false);
  const [openEditModal, setOpenEditModal] = useState(false);
  const [selectedNotice, setSelectedNotice] = useState<Notice | undefined>();

  const {
    register: registerCreate,
    handleSubmit: handleCreateSubmit,
    reset: resetCreate,
    setValue: setValueCreate,
    watch: watchCreate,
  } = useForm<NoticeFormData>();

  const {
    register: registerEdit,
    handleSubmit: handleEditSubmit,
    reset: resetEdit,
    setValue: setValueEdit,
    watch: watchEdit,
  } = useForm<Partial<NoticeFormData>>();

  // Fetch notices
  const { data: notices } = getNotices({});

  const handleDelete = async (id: string) => {
    if (window.confirm("Are you sure you want to delete this notice?")) {
      try {
        await deleteNotice({ params: { path: { id } } }).mutate();
        await queryClient.invalidateQueries({ queryKey: ["/notices/"] });
        toast.success("Notice deleted successfully");
      } catch {
        toast.error("Failed to delete notice");
      }
    }
  };

  const onCreateSubmit = async (data: NoticeFormData) => {
    try {
      // Convert datetime-local to NaiveDateTime format (strip timezone from ISO string)
      // toISOString() gives "2024-01-15T10:30:00.000Z", slice to get "2024-01-15T10:30:00"
      const formattedData = {
        ...data,
        published_at: new Date(data.published_at).toISOString().slice(0, 19),
      };
      await createNotice({ body: formattedData }).mutate();
      await queryClient.invalidateQueries({ queryKey: ["/notices/"] });
      toast.success("Notice created successfully");
      setOpenCreateModal(false);
      resetCreate();
    } catch (error: any) {
      const message = error?.message || "Failed to create notice";
      toast.error(message);
    }
  };

  const onEditSubmit = async (data: Partial<NoticeFormData>) => {
    if (selectedNotice) {
      try {
        // Convert datetime-local to NaiveDateTime format if published_at is provided
        const formattedData = {
          ...data,
          ...(data.published_at && {
            published_at: new Date(data.published_at)
              .toISOString()
              .slice(0, 19),
          }),
        };
        await updateNotice({
          params: { path: { id: selectedNotice.id } },
          body: formattedData,
        }).mutate();
        await queryClient.invalidateQueries({ queryKey: ["/notices/"] });
        toast.success("Notice updated successfully");
        setOpenEditModal(false);
        resetEdit();
      } catch (error: any) {
        const message = error?.message || "Failed to update notice";
        toast.error(message);
      }
    }
  };

  return (
    <>
      <div className="p-4">
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-2xl font-bold">Notices</h1>
          <Button onClick={() => setOpenCreateModal(true)}>
            <FaPlus className="mr-2" />
            Create Notice
          </Button>
        </div>

        <Table>
          <Table.Head>
            <Table.HeadCell>Title</Table.HeadCell>
            <Table.HeadCell>Slug</Table.HeadCell>
            <Table.HeadCell>Published At</Table.HeadCell>
            <Table.HeadCell>Actions</Table.HeadCell>
          </Table.Head>
          <Table.Body>
            {notices?.map((notice) => (
              <Table.Row key={notice.id}>
                <Table.Cell>{notice.title}</Table.Cell>
                <Table.Cell>
                  <code className="text-sm text-gray-600">{notice.slug}</code>
                </Table.Cell>
                <Table.Cell>
                  {new Date(notice.published_at).toLocaleString()}
                </Table.Cell>
                <Table.Cell>
                  <div className="flex gap-2">
                    <Button
                      size="xs"
                      onClick={() => {
                        setSelectedNotice(notice);
                        setOpenEditModal(true);
                      }}
                    >
                      <FaEdit />
                    </Button>
                    <Button
                      size="xs"
                      color="failure"
                      onClick={() => handleDelete(notice.id)}
                    >
                      <FaTrash />
                    </Button>
                  </div>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      </div>

      {/* Create Modal */}
      <Modal show={openCreateModal} onClose={() => setOpenCreateModal(false)}>
        <Modal.Header>Create Notice</Modal.Header>
        <Modal.Body>
          <form onSubmit={handleCreateSubmit(onCreateSubmit)}>
            <div className="flex flex-col gap-4">
              <div>
                <Label>Title</Label>
                <TextInput
                  {...registerCreate("title", { required: true })}
                  placeholder="Notice title..."
                  required
                />
              </div>
              <div>
                <Label>Slug</Label>
                <div className="flex gap-2">
                  <TextInput
                    {...registerCreate("slug", { required: true })}
                    placeholder="url-friendly-slug"
                    required
                    className="flex-1"
                  />
                  <Button
                    type="button"
                    color="gray"
                    size="sm"
                    onClick={() => {
                      const title = watchCreate("title");
                      if (title) {
                        setValueCreate("slug", generateSlug(title));
                      }
                    }}
                  >
                    <FaSync className="m-1 ml-0 mr-2" />
                    <span>Generate</span>
                  </Button>
                </div>
              </div>
              <div>
                <Label>Content</Label>
                <Textarea
                  {...registerCreate("content", { required: true })}
                  placeholder="Notice content..."
                  required
                  rows={6}
                />
              </div>
              <div>
                <Label>Published At</Label>
                <TextInput
                  {...registerCreate("published_at", { required: true })}
                  type="datetime-local"
                  required
                />
              </div>
              <Button type="submit">Create</Button>
            </div>
          </form>
        </Modal.Body>
      </Modal>

      {/* Edit Modal */}
      <Modal show={openEditModal} onClose={() => setOpenEditModal(false)}>
        <Modal.Header>Edit Notice</Modal.Header>
        <Modal.Body>
          <form onSubmit={handleEditSubmit(onEditSubmit)}>
            <div className="flex flex-col gap-4">
              <div>
                <Label>Title</Label>
                <TextInput
                  {...registerEdit("title")}
                  defaultValue={selectedNotice?.title}
                  placeholder="Notice title..."
                />
              </div>
              <div>
                <Label>Slug</Label>
                <div className="flex gap-2">
                  <TextInput
                    {...registerEdit("slug")}
                    defaultValue={selectedNotice?.slug}
                    placeholder="url-friendly-slug"
                    className="flex-1"
                  />
                  <Button
                    type="button"
                    color="gray"
                    size="sm"
                    className="items-center"
                    onClick={() => {
                      const title = watchEdit("title") || selectedNotice?.title;
                      if (title) {
                        setValueEdit("slug", generateSlug(title));
                      }
                    }}
                  >
                    <FaSync className="m-1 ml-0 mr-2" />
                    <span>Generate</span>
                  </Button>
                </div>
              </div>
              <div>
                <Label>Content</Label>
                <Textarea
                  {...registerEdit("content")}
                  defaultValue={selectedNotice?.content}
                  placeholder="Notice content..."
                  rows={6}
                />
              </div>
              <div>
                <Label>Published At</Label>
                <TextInput
                  {...registerEdit("published_at")}
                  type="datetime-local"
                  defaultValue={
                    selectedNotice?.published_at
                      ? new Date(selectedNotice.published_at)
                          .toISOString()
                          .slice(0, 16)
                      : ""
                  }
                />
              </div>
              <Button type="submit">Update</Button>
            </div>
          </form>
        </Modal.Body>
      </Modal>
    </>
  );
};

export default Notices;
