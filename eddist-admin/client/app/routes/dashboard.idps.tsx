import {
  Button,
  Checkbox,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
  Textarea,
  TextInput,
} from "flowbite-react";
import { useForm } from "react-hook-form";
import { FaEdit, FaPlus, FaTrash } from "react-icons/fa";
import { getIdps, useCreateIdp, useDeleteIdp, useUpdateIdp } from "~/hooks/queries";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import type { paths } from "~/openapi/schema";

type Idp = paths["/idps/"]["get"]["responses"]["200"]["content"]["application/json"][number];

type CreateIdpFormData = paths["/idps/"]["post"]["requestBody"]["content"]["application/json"];

type UpdateIdpFormData =
  paths["/idps/{id}/"]["patch"]["requestBody"]["content"]["application/json"];

const IdpActions = ({
  idp,
  onEdit,
  onDelete,
}: {
  idp: Idp;
  onEdit: () => void;
  onDelete: () => void;
}) => (
  <div className="flex shrink-0 gap-2">
    <Button
      size="xs"
      className="min-h-10 min-w-10 justify-center p-2"
      onClick={onEdit}
      aria-label={`Edit ${idp.idp_display_name}`}
    >
      <FaEdit aria-hidden="true" />
      <span className="sr-only">Edit</span>
    </Button>
    <Button
      size="xs"
      color="alternative"
      className="min-h-10 min-w-10 justify-center p-2"
      onClick={onDelete}
      aria-label={`Delete ${idp.idp_display_name}`}
    >
      <FaTrash aria-hidden="true" />
      <span className="sr-only">Delete</span>
    </Button>
  </div>
);

function decodeBase64Svg(b64: string | null | undefined): string {
  if (!b64) return "";
  try {
    return atob(b64);
  } catch {
    return b64;
  }
}

function encodeBase64Svg(raw: string | null | undefined): string | undefined {
  if (!raw) return undefined;
  return btoa(raw);
}

interface IdpFormProps {
  mode: "create" | "edit";
  defaultValues?: Idp;
  onSubmit: (data: CreateIdpFormData | UpdateIdpFormData) => void;
}

const IdpForm = ({ mode, defaultValues, onSubmit }: IdpFormProps) => {
  const isCreate = mode === "create";
  const { register, handleSubmit, reset, watch } = useForm<CreateIdpFormData>();

  const decodedDefault = decodeBase64Svg(defaultValues?.idp_logo_svg);
  const svgValue = watch("idp_logo_svg", decodedDefault);

  return (
    <form
      onSubmit={handleSubmit((data) => {
        // Encode raw SVG to base64 before sending
        const encoded = {
          ...data,
          idp_logo_svg: encodeBase64Svg(data.idp_logo_svg),
        };
        if (!isCreate) {
          // For update, remove empty client_secret so backend keeps current
          const updateData: UpdateIdpFormData = { ...encoded };
          if (!updateData.client_secret) {
            delete updateData.client_secret;
          }
          onSubmit(updateData);
        } else {
          onSubmit(encoded);
        }
        reset();
      })}
    >
      <div className="flex flex-col gap-4">
        <div>
          <Label>IdP Name</Label>
          <TextInput
            {...register("idp_name", { required: isCreate })}
            defaultValue={defaultValues?.idp_name}
            placeholder="e.g. google, github"
            required={isCreate}
            disabled={!isCreate}
          />
        </div>
        <div>
          <Label>Display Name</Label>
          <TextInput
            {...register("idp_display_name", { required: isCreate })}
            defaultValue={defaultValues?.idp_display_name}
            placeholder="e.g. Google, GitHub"
            required={isCreate}
          />
        </div>
        <div>
          <Label>Logo SVG</Label>
          <Textarea
            {...register("idp_logo_svg")}
            defaultValue={decodedDefault}
            placeholder="<svg>...</svg>"
            rows={3}
          />
          {/* Raw render is acceptable here since only trusted admins access this page */}
          {svgValue && (
            <div className="mt-2 p-2 border border-gray-200 rounded bg-white">
              <Label className="text-xs text-gray-500 mb-1 block">Preview</Label>
              <div
                className="flex items-center justify-center [&>svg]:max-h-12 [&>svg]:max-w-full"
                // biome-ignore lint/security/noDangerouslySetInnerHtml: SVG preview for trusted admin input
                dangerouslySetInnerHTML={{ __html: svgValue }}
              />
            </div>
          )}
        </div>
        <div>
          <Label>OIDC Config URL</Label>
          <TextInput
            {...register("oidc_config_url", { required: isCreate })}
            defaultValue={defaultValues?.oidc_config_url}
            placeholder="https://accounts.google.com/.well-known/openid-configuration"
            required={isCreate}
          />
        </div>
        <div>
          <Label>Client ID</Label>
          <TextInput
            {...register("client_id", { required: isCreate })}
            defaultValue={defaultValues?.client_id}
            placeholder="Client ID"
            required={isCreate}
          />
        </div>
        <div>
          <Label>Client Secret</Label>
          <TextInput
            {...register("client_secret", { required: isCreate })}
            type="password"
            placeholder={isCreate ? "Client secret" : "Leave empty to keep current"}
            required={isCreate}
          />
        </div>
        <div className="flex items-center gap-2">
          <Checkbox
            {...register("enabled")}
            id="enabled"
            defaultChecked={defaultValues?.enabled ?? true}
          />
          <Label htmlFor="enabled">Enabled</Label>
        </div>
        <Button type="submit">{isCreate ? "Create" : "Update"}</Button>
      </div>
    </form>
  );
};

const IdPs = () => {
  const modal = useCrudModalState<Idp>();

  const createMutation = useCreateIdp();
  const updateMutation = useUpdateIdp();
  const deleteMutation = useDeleteIdp();

  const { data: idps } = getIdps({});

  const handleDelete = (id: string) => {
    if (window.confirm("Are you sure you want to delete this IdP?")) {
      deleteMutation.mutate({ params: { path: { id } } });
    }
  };

  return (
    <>
      <div className="mx-auto w-full max-w-7xl p-4 sm:p-6 lg:p-8">
        <div className="mb-5 flex flex-col gap-3 border-b border-gray-200 pb-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900 sm:text-3xl">Identity Providers</h1>
            <p className="mt-1 text-sm text-gray-500">
              Manage the external identity providers available to users.
            </p>
          </div>
          <Button className="w-full sm:w-auto" onClick={() => modal.openCreate()}>
            <FaPlus className="mr-2" />
            Create IdP
          </Button>
        </div>

        <div className="hidden overflow-x-auto rounded-xl border border-gray-200 bg-white md:block">
          <Table hoverable className="min-w-[700px]">
            <TableHead>
              <TableHeadCell>Name</TableHeadCell>
              <TableHeadCell>Display Name</TableHeadCell>
              <TableHeadCell>OIDC Config URL</TableHeadCell>
              <TableHeadCell>Enabled</TableHeadCell>
              <TableHeadCell>Actions</TableHeadCell>
            </TableHead>
            <TableBody>
              {idps?.map((idp) => (
                <TableRow className="border-gray-200" key={idp.id}>
                  <TableCell>
                    <code className="text-sm text-gray-600">{idp.idp_name}</code>
                  </TableCell>
                  <TableCell className="font-medium">{idp.idp_display_name}</TableCell>
                  <TableCell>
                    <span className="block max-w-xs truncate text-sm">{idp.oidc_config_url}</span>
                  </TableCell>
                  <TableCell>
                    <span className={idp.enabled ? "text-green-500" : "text-red-500"}>
                      {idp.enabled ? "Yes" : "No"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <IdpActions
                      idp={idp}
                      onEdit={() => modal.openEdit(idp)}
                      onDelete={() => handleDelete(idp.id)}
                    />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <div className="space-y-3 md:hidden">
          {idps?.map((idp) => (
            <article
              key={idp.id}
              className="rounded-xl border border-gray-200 bg-white p-4 shadow-sm"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="font-mono text-sm text-gray-500">{idp.idp_name}</p>
                  <h2 className="mt-1 break-words font-semibold text-gray-900">
                    {idp.idp_display_name}
                  </h2>
                </div>
                <IdpActions
                  idp={idp}
                  onEdit={() => modal.openEdit(idp)}
                  onDelete={() => handleDelete(idp.id)}
                />
              </div>

              <dl className="mt-4 space-y-3 border-t border-gray-100 pt-4">
                <div className="min-w-0">
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    OIDC config URL
                  </dt>
                  <dd className="mt-1 break-all text-sm text-gray-700">{idp.oidc_config_url}</dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Status
                  </dt>
                  <dd
                    className={
                      idp.enabled
                        ? "mt-1 font-medium text-green-600"
                        : "mt-1 font-medium text-red-600"
                    }
                  >
                    {idp.enabled ? "Enabled" : "Disabled"}
                  </dd>
                </div>
              </dl>
            </article>
          ))}
          {idps?.length === 0 && (
            <div className="rounded-xl border border-dashed border-gray-300 bg-white p-8 text-center text-sm text-gray-500">
              No identity providers found.
            </div>
          )}
        </div>
      </div>

      {/* Create Modal */}
      <Modal show={modal.isCreateOpen} onClose={() => modal.closeCreate()} dismissible>
        <ModalHeader className="border-gray-200">Create IdP</ModalHeader>
        <ModalBody>
          <IdpForm
            mode="create"
            onSubmit={(data) => {
              createMutation.mutate(
                { body: data as CreateIdpFormData },
                { onSuccess: () => modal.closeCreate() },
              );
            }}
          />
        </ModalBody>
      </Modal>

      {/* Edit Modal */}
      {modal.editingItem && (
        <Modal show={modal.isEditOpen} onClose={() => modal.closeEdit()} dismissible>
          <ModalHeader className="border-gray-200">Edit IdP</ModalHeader>
          <ModalBody>
            <IdpForm
              mode="edit"
              defaultValues={modal.editingItem}
              onSubmit={(data) => {
                updateMutation.mutate(
                  {
                    params: { path: { id: modal.editingItem?.id ?? "" } },
                    body: data as UpdateIdpFormData,
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

export default IdPs;
