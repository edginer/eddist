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
import CaptchaConfigForm from "~/components/CaptchaConfigForm";
import {
  getCaptchaConfigs,
  useCreateCaptchaConfig,
  useDeleteCaptchaConfig,
  useUpdateCaptchaConfig,
} from "~/hooks/queries";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import type { paths } from "~/openapi/schema";

type CaptchaConfig =
  paths["/captcha-configs/"]["get"]["responses"]["200"]["content"]["application/json"][number];

const getProviderBadgeColor = (provider: string) => {
  switch (provider.toLowerCase()) {
    case "turnstile":
      return "warning";
    case "hcaptcha":
      return "info";
    case "monocle":
      return "purple";
    case "layer3intel_tripwire":
      return "green";
    default:
      return "gray";
  }
};

const getEndpointBadgeColor = (endpointUsage: string) => {
  if (endpointUsage === "re_auth") return "indigo";
  if (endpointUsage === "all") return "purple";
  return "blue";
};

const ConfigActions = ({
  config,
  onEdit,
  onDelete,
}: {
  config: CaptchaConfig;
  onEdit: () => void;
  onDelete: () => void;
}) => (
  <div className="flex shrink-0 gap-2">
    <Button
      size="xs"
      className="min-h-10 min-w-10 justify-center p-2"
      onClick={onEdit}
      aria-label={`Edit ${config.name}`}
    >
      <FaEdit aria-hidden="true" />
      <span className="sr-only">Edit</span>
    </Button>
    <Button
      size="xs"
      color="alternative"
      className="min-h-10 min-w-10 justify-center p-2"
      onClick={onDelete}
      aria-label={`Delete ${config.name}`}
    >
      <FaTrash aria-hidden="true" />
      <span className="sr-only">Delete</span>
    </Button>
  </div>
);

const CaptchaConfigs = () => {
  const modal = useCrudModalState<CaptchaConfig>();

  const { data: configs } = getCaptchaConfigs();

  const createMutation = useCreateCaptchaConfig();
  const updateMutation = useUpdateCaptchaConfig();
  const deleteMutation = useDeleteCaptchaConfig();

  const handleDelete = (id: string) => {
    if (window.confirm("Are you sure you want to delete this captcha config?")) {
      deleteMutation.mutate({ params: { path: { id } } });
    }
  };

  return (
    <>
      <div className="mx-auto w-full max-w-7xl p-4 sm:p-6 lg:p-8">
        <div className="mb-5 flex flex-col gap-3 border-b border-gray-200 pb-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900 sm:text-3xl">Captcha Configs</h1>
            <p className="mt-1 text-sm text-gray-500">
              Configure the providers used to protect posting endpoints.
            </p>
          </div>
          <Button className="w-full sm:w-auto" onClick={() => modal.openCreate()}>
            <FaPlus className="mr-2" />
            Create Config
          </Button>
        </div>

        <div className="hidden overflow-x-auto rounded-xl border border-gray-200 bg-white md:block">
          <Table hoverable className="min-w-[760px]">
            <TableHead>
              <TableHeadCell>Name</TableHeadCell>
              <TableHeadCell>Provider</TableHeadCell>
              <TableHeadCell>Site Key</TableHeadCell>
              <TableHeadCell>Endpoint</TableHeadCell>
              <TableHeadCell>Status</TableHeadCell>
              <TableHeadCell>Order</TableHeadCell>
              <TableHeadCell>Actions</TableHeadCell>
            </TableHead>
            <TableBody>
              {configs?.map((config) => (
                <TableRow className="border-gray-200" key={config.id}>
                  <TableCell className="font-medium">{config.name}</TableCell>
                  <TableCell>
                    <Badge color={getProviderBadgeColor(config.provider)}>{config.provider}</Badge>
                  </TableCell>
                  <TableCell>
                    <code className="block max-w-xs truncate text-sm text-gray-600">
                      {config.site_key}
                    </code>
                  </TableCell>
                  <TableCell>
                    <Badge color={getEndpointBadgeColor(config.endpoint_usage)}>
                      {config.endpoint_usage}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Badge color={config.is_active ? "success" : "gray"}>
                      {config.is_active ? "Active" : "Inactive"}
                    </Badge>
                  </TableCell>
                  <TableCell>{config.display_order}</TableCell>
                  <TableCell>
                    <ConfigActions
                      config={config}
                      onEdit={() => modal.openEdit(config)}
                      onDelete={() => handleDelete(config.id)}
                    />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="space-y-3 md:hidden">
          {configs?.map((config) => (
            <article
              key={config.id}
              className="rounded-xl border border-gray-200 bg-white p-4 shadow-sm"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <h2 className="break-words font-semibold text-gray-900">{config.name}</h2>
                  <div className="mt-2 flex flex-wrap gap-2">
                    <Badge color={getProviderBadgeColor(config.provider)}>{config.provider}</Badge>
                    <Badge color={config.is_active ? "success" : "gray"}>
                      {config.is_active ? "Active" : "Inactive"}
                    </Badge>
                  </div>
                </div>
                <ConfigActions
                  config={config}
                  onEdit={() => modal.openEdit(config)}
                  onDelete={() => handleDelete(config.id)}
                />
              </div>

              <dl className="mt-4 grid grid-cols-1 gap-3 border-t border-gray-100 pt-4 sm:grid-cols-2">
                <div className="min-w-0">
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Site key
                  </dt>
                  <dd className="mt-1 break-all font-mono text-sm text-gray-700">
                    {config.site_key}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Endpoint
                  </dt>
                  <dd className="mt-1">
                    <Badge color={getEndpointBadgeColor(config.endpoint_usage)}>
                      {config.endpoint_usage}
                    </Badge>
                  </dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Display order
                  </dt>
                  <dd className="mt-1 text-sm text-gray-700">{config.display_order}</dd>
                </div>
              </dl>
            </article>
          ))}
          {configs?.length === 0 && (
            <div className="rounded-xl border border-dashed border-gray-300 bg-white p-8 text-center text-sm text-gray-500">
              No captcha configurations found.
            </div>
          )}
        </div>
      </div>

      {/* Create Modal */}
      <Modal show={modal.isCreateOpen} onClose={() => modal.closeCreate()} size="xl" dismissible>
        <ModalHeader className="border-gray-200">Create Captcha Config</ModalHeader>
        <ModalBody>
          <CaptchaConfigForm
            mode="create"
            onSubmit={(data) => {
              createMutation.mutate({ body: data }, { onSuccess: () => modal.closeCreate() });
            }}
          />
        </ModalBody>
      </Modal>

      {/* Edit Modal */}
      {modal.editingItem && (
        <Modal show={modal.isEditOpen} onClose={() => modal.closeEdit()} size="xl" dismissible>
          <ModalHeader className="border-gray-200">Edit Captcha Config</ModalHeader>
          <ModalBody>
            <CaptchaConfigForm
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

export default CaptchaConfigs;
