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
import { FaPlus, FaEdit, FaTrash } from "react-icons/fa";
import { useForm } from "react-hook-form";
import {
  getIdps,
  useCreateIdp,
  useUpdateIdp,
  useDeleteIdp,
} from "~/hooks/queries";
import type { paths } from "~/openapi/schema";
import { useCrudModalState } from "~/hooks/useCrudModalState";

type Idp =
  paths["/idps/"]["get"]["responses"]["200"]["content"]["application/json"][number];

type CreateIdpFormData =
  paths["/idps/"]["post"]["requestBody"]["content"]["application/json"];

type UpdateIdpFormData =
  paths["/idps/{id}/"]["patch"]["requestBody"]["content"]["application/json"];

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
  const { register, handleSubmit, reset, watch } =
    useForm<CreateIdpFormData>();

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
              <Label className="text-xs text-gray-500 mb-1 block">
                Preview
              </Label>
              <div
                className="flex items-center justify-center [&>svg]:max-h-12 [&>svg]:max-w-full"
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
            placeholder={
              isCreate ? "Client secret" : "Leave empty to keep current"
            }
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
      <div className="p-4">
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-2xl font-bold">Identity Providers</h1>
          <Button onClick={() => modal.openCreate()}>
            <FaPlus className="mr-2" />
            Create IdP
          </Button>
        </div>

        <Table>
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
                <TableCell>{idp.idp_display_name}</TableCell>
                <TableCell>
                  <span className="text-sm truncate max-w-xs block">
                    {idp.oidc_config_url}
                  </span>
                </TableCell>
                <TableCell>
                  <span
                    className={idp.enabled ? "text-green-500" : "text-red-500"}
                  >
                    {idp.enabled ? "Yes" : "No"}
                  </span>
                </TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button size="xs" onClick={() => modal.openEdit(idp)}>
                      <FaEdit />
                    </Button>
                    <Button
                      size="xs"
                      color="alternative"
                      onClick={() => handleDelete(idp.id)}
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
      <Modal show={modal.isCreateOpen} onClose={() => modal.closeCreate()}>
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
        <Modal show={modal.isEditOpen} onClose={() => modal.closeEdit()}>
          <ModalHeader className="border-gray-200">Edit IdP</ModalHeader>
          <ModalBody>
            <IdpForm
              mode="edit"
              defaultValues={modal.editingItem}
              onSubmit={(data) => {
                updateMutation.mutate(
                  {
                    params: { path: { id: modal.editingItem!.id } },
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
