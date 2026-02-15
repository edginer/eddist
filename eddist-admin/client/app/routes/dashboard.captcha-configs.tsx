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
import { FaPlus, FaEdit, FaTrash } from "react-icons/fa";
import {
  getCaptchaConfigs,
  useCreateCaptchaConfig,
  useUpdateCaptchaConfig,
  useDeleteCaptchaConfig,
} from "~/hooks/queries";
import type { paths } from "~/openapi/schema";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import CaptchaConfigForm from "~/components/CaptchaConfigForm";

type CaptchaConfig =
  paths["/captcha-configs/"]["get"]["responses"]["200"]["content"]["application/json"][number];

const CaptchaConfigs = () => {
  const modal = useCrudModalState<CaptchaConfig>();

  const { data: configs } = getCaptchaConfigs();

  const createMutation = useCreateCaptchaConfig();
  const updateMutation = useUpdateCaptchaConfig();
  const deleteMutation = useDeleteCaptchaConfig();

  const handleDelete = (id: string) => {
    if (
      window.confirm("Are you sure you want to delete this captcha config?")
    ) {
      deleteMutation.mutate({ params: { path: { id } } });
    }
  };

  const getProviderBadgeColor = (provider: string) => {
    switch (provider.toLowerCase()) {
      case "turnstile":
        return "warning";
      case "hcaptcha":
        return "info";
      case "monocle":
        return "purple";
      default:
        return "gray";
    }
  };

  return (
    <>
      <div className="p-4">
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-2xl font-bold">Captcha Configs</h1>
          <Button onClick={() => modal.openCreate()}>
            <FaPlus className="mr-2" />
            Create Config
          </Button>
        </div>

        <Table>
          <TableHead>
            <TableHeadCell>Name</TableHeadCell>
            <TableHeadCell>Provider</TableHeadCell>
            <TableHeadCell>Site Key</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell>Order</TableHeadCell>
            <TableHeadCell>Actions</TableHeadCell>
          </TableHead>
          <TableBody>
            {configs?.map((config) => (
              <TableRow className="border-gray-200" key={config.id}>
                <TableCell>{config.name}</TableCell>
                <TableCell>
                  <Badge color={getProviderBadgeColor(config.provider)}>
                    {config.provider}
                  </Badge>
                </TableCell>
                <TableCell>
                  <code className="text-sm text-gray-600 max-w-xs truncate block">
                    {config.site_key}
                  </code>
                </TableCell>
                <TableCell>
                  <Badge color={config.is_active ? "success" : "gray"}>
                    {config.is_active ? "Active" : "Inactive"}
                  </Badge>
                </TableCell>
                <TableCell>{config.display_order}</TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button
                      size="xs"
                      onClick={() => modal.openEdit(config)}
                    >
                      <FaEdit />
                    </Button>
                    <Button
                      size="xs"
                      color="alternative"
                      onClick={() => handleDelete(config.id)}
                    >
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
      <Modal
        show={modal.isCreateOpen}
        onClose={() => modal.closeCreate()}
        size="xl"
      >
        <ModalHeader className="border-gray-200">
          Create Captcha Config
        </ModalHeader>
        <ModalBody>
          <CaptchaConfigForm
            mode="create"
            onSubmit={(data) => {
              createMutation.mutate(
                { body: data },
                { onSuccess: () => modal.closeCreate() },
              );
            }}
          />
        </ModalBody>
      </Modal>

      {/* Edit Modal */}
      {modal.editingItem && (
        <Modal
          show={modal.isEditOpen}
          onClose={() => modal.closeEdit()}
          size="xl"
        >
          <ModalHeader className="border-gray-200">
            Edit Captcha Config
          </ModalHeader>
          <ModalBody>
            <CaptchaConfigForm
              mode="edit"
              defaultValues={modal.editingItem}
              onSubmit={(data) => {
                updateMutation.mutate(
                  {
                    params: { path: { id: modal.editingItem!.id } },
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

export default CaptchaConfigs;
