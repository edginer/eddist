import {
  Button,
  Label,
  Modal,
  Table,
  Textarea,
  TextInput,
} from "flowbite-react";
import { useState } from "react";
import { FaPlus, FaEdit, FaTrash } from "react-icons/fa";
import { useForm } from "react-hook-form";
import { toast } from "react-toastify";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

interface Notice {
  id: string;
  title: string;
  content: string;
  summary: string | null;
  created_at: string;
  updated_at: string;
  published_at: string;
  author_id: string | null;
}

interface NoticeFormData {
  title: string;
  content: string;
  summary?: string;
  published_at: string;
}

const Notices = () => {
  const queryClient = useQueryClient();
  const [openCreateModal, setOpenCreateModal] = useState(false);
  const [openEditModal, setOpenEditModal] = useState(false);
  const [selectedNotice, setSelectedNotice] = useState<Notice | undefined>();
  const { register: registerCreate, handleSubmit: handleCreateSubmit, reset: resetCreate } = useForm<NoticeFormData>();
  const { register: registerEdit, handleSubmit: handleEditSubmit, reset: resetEdit } = useForm<NoticeFormData>();

  // Fetch notices
  const { data: notices, refetch } = useQuery({
    queryKey: ["notices"],
    queryFn: async () => {
      const response = await fetch("/api/notices");
      if (!response.ok) throw new Error("Failed to fetch notices");
      return response.json() as Promise<Notice[]>;
    },
  });

  // Create notice mutation
  const createMutation = useMutation({
    mutationFn: async (data: NoticeFormData) => {
      const response = await fetch("/api/notices", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data),
      });
      if (!response.ok) throw new Error("Failed to create notice");
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["notices"] });
      toast.success("Notice created successfully");
      setOpenCreateModal(false);
      resetCreate();
    },
    onError: () => {
      toast.error("Failed to create notice");
    },
  });

  // Update notice mutation
  const updateMutation = useMutation({
    mutationFn: async ({ id, data }: { id: string; data: Partial<NoticeFormData> }) => {
      const response = await fetch(`/api/notices/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data),
      });
      if (!response.ok) throw new Error("Failed to update notice");
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["notices"] });
      toast.success("Notice updated successfully");
      setOpenEditModal(false);
      resetEdit();
    },
    onError: () => {
      toast.error("Failed to update notice");
    },
  });

  // Delete notice mutation
  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const response = await fetch(`/api/notices/${id}`, {
        method: "DELETE",
      });
      if (!response.ok) throw new Error("Failed to delete notice");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["notices"] });
      toast.success("Notice deleted successfully");
    },
    onError: () => {
      toast.error("Failed to delete notice");
    },
  });

  const handleDelete = async (id: string) => {
    if (window.confirm("Are you sure you want to delete this notice?")) {
      deleteMutation.mutate(id);
    }
  };

  const onCreateSubmit = (data: NoticeFormData) => {
    createMutation.mutate(data);
  };

  const onEditSubmit = (data: NoticeFormData) => {
    if (selectedNotice) {
      updateMutation.mutate({ id: selectedNotice.id, data });
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
            <Table.HeadCell>Published At</Table.HeadCell>
            <Table.HeadCell>Actions</Table.HeadCell>
          </Table.Head>
          <Table.Body>
            {notices?.map((notice) => (
              <Table.Row key={notice.id}>
                <Table.Cell>{notice.title}</Table.Cell>
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
                <Label>Summary (optional)</Label>
                <TextInput
                  {...registerCreate("summary")}
                  placeholder="Brief summary..."
                />
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
                <Label>Summary (optional)</Label>
                <TextInput
                  {...registerEdit("summary")}
                  defaultValue={selectedNotice?.summary || ""}
                  placeholder="Brief summary..."
                />
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
