import {
  Dropdown,
  DropdownItem,
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
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import RestrictionRuleForm from "~/components/RestrictionRuleForm";
import {
  getRestrictionRules,
  useCreateRestrictionRule,
  useDeleteRestrictionRule,
  useUpdateRestrictionRule,
} from "~/hooks/queries";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import { formatDateTime } from "~/utils/format";

interface RestrictionRule {
  id: string;
  name: string;
  rule_type: "Asn" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string | null;
  created_at: string;
  updated_at: string;
  created_by_email: string;
}

const RestrictionRules = () => {
  const { data: restrictionRules } = getRestrictionRules({});
  const createMutation = useCreateRestrictionRule();
  const updateMutation = useUpdateRestrictionRule();
  const deleteMutation = useDeleteRestrictionRule();
  const modal = useCrudModalState<RestrictionRule>();

  const formatExpiry = (expiresAt?: string | null) => {
    if (!expiresAt) return "Never";
    const expiry = new Date(expiresAt);
    const now = new Date();
    if (expiry < now) return "Expired";
    return expiry.toLocaleString();
  };

  return (
    <>
      <Modal show={modal.isCreateOpen} onClose={() => modal.closeCreate()} dismissible>
        <ModalHeader className="border-gray-200">Create Restriction Rule</ModalHeader>
        <ModalBody>
          <RestrictionRuleForm
            mode="create"
            onSubmit={(data) => {
              createMutation.mutate({ body: data }, { onSuccess: () => modal.closeCreate() });
            }}
          />
        </ModalBody>
      </Modal>

      {modal.editingItem && (
        <Modal show={modal.isEditOpen} onClose={() => modal.closeEdit()} dismissible>
          <ModalHeader className="border-gray-200">Edit Restriction Rule</ModalHeader>
          <ModalBody>
            <RestrictionRuleForm
              mode="edit"
              defaultValues={modal.editingItem}
              onSubmit={(data) => {
                updateMutation.mutate(
                  {
                    params: { path: { rule_id: modal.editingItem?.id ?? "" } },
                    body: data,
                  },
                  { onSuccess: () => modal.closeEdit() },
                );
              }}
            />
          </ModalBody>
        </Modal>
      )}

      <div className="p-4 sm:p-6 lg:p-8">
        <div className="flex items-center gap-3">
          <h1 className="grow text-2xl font-bold sm:text-3xl">Restriction Rules</h1>
          <button
            type="button"
            className="min-h-11 min-w-11 shrink-0 rounded-xl bg-slate-400 p-3 shadow-lg hover:bg-slate-500"
            aria-label="Create restriction rule"
            onClick={() => modal.openCreate()}
          >
            <FaPlus />
          </button>
        </div>
        <div className="mt-4 hidden overflow-x-auto md:block">
          <Table className="min-w-[980px]">
            <TableHead>
              <TableHeadCell>Name</TableHeadCell>
              <TableHeadCell>Type</TableHeadCell>
              <TableHeadCell>Value</TableHeadCell>
              <TableHeadCell>Expires</TableHeadCell>
              <TableHeadCell>Created By</TableHeadCell>
              <TableHeadCell>Created At</TableHeadCell>
              <TableHeadCell></TableHeadCell>
            </TableHead>
            <TableBody className="divide-y">
              {restrictionRules?.map((rule) => (
                <TableRow className="border-gray-200" key={rule.id}>
                  <TableCell className="font-medium">{rule.name}</TableCell>
                  <TableCell>
                    <span className="rounded-full bg-blue-100 px-2 py-1 text-xs font-semibold text-blue-800">
                      {rule.rule_type}
                    </span>
                  </TableCell>
                  <TableCell className="font-mono text-sm">{rule.rule_value}</TableCell>
                  <TableCell>
                    <span
                      className={`rounded-full px-2 py-1 text-xs font-semibold ${
                        rule.expires_at
                          ? new Date(rule.expires_at) < new Date()
                            ? "bg-red-100 text-red-800"
                            : "bg-yellow-100 text-yellow-800"
                          : "bg-green-100 text-green-800"
                      }`}
                    >
                      {formatExpiry(rule.expires_at)}
                    </span>
                  </TableCell>
                  <TableCell>{rule.created_by_email}</TableCell>
                  <TableCell>{formatDateTime(rule.created_at)}</TableCell>
                  <TableCell>
                    <div className="text-right">
                      <Dropdown label={<BiDotsHorizontalRounded />}>
                        <DropdownItem onClick={() => modal.openEdit(rule)}>Edit</DropdownItem>
                        <DropdownItem
                          className="text-red-500"
                          onClick={() => {
                            deleteMutation.mutate({
                              params: {
                                path: {
                                  rule_id: rule.id,
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
          {restrictionRules?.map((rule) => (
            <article
              key={rule.id}
              className="rounded-xl border border-gray-200 bg-white p-4 shadow-sm"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <h2 className="break-words font-semibold text-gray-900">{rule.name}</h2>
                  <span className="mt-2 inline-block rounded-full bg-blue-100 px-2 py-1 text-xs font-semibold text-blue-800">
                    {rule.rule_type}
                  </span>
                </div>
                <Dropdown label={<BiDotsHorizontalRounded />}>
                  <DropdownItem onClick={() => modal.openEdit(rule)}>Edit</DropdownItem>
                  <DropdownItem
                    className="text-red-500"
                    onClick={() =>
                      deleteMutation.mutate({ params: { path: { rule_id: rule.id } } })
                    }
                  >
                    Delete
                  </DropdownItem>
                </Dropdown>
              </div>
              <dl className="mt-4 space-y-3 border-t border-gray-100 pt-4">
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Value
                  </dt>
                  <dd className="mt-1 break-all font-mono text-sm text-gray-700">
                    {rule.rule_value}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Expires
                  </dt>
                  <dd className="mt-1 text-sm text-gray-700">{formatExpiry(rule.expires_at)}</dd>
                </div>
                <div>
                  <dt className="text-xs font-medium uppercase tracking-wide text-gray-500">
                    Created
                  </dt>
                  <dd className="mt-1 break-words text-sm text-gray-700">
                    {rule.created_by_email} · {formatDateTime(rule.created_at)}
                  </dd>
                </div>
              </dl>
            </article>
          ))}
          {restrictionRules?.length === 0 && (
            <div className="rounded-xl border border-dashed border-gray-300 bg-white p-8 text-center text-sm text-gray-500">
              No restriction rules found.
            </div>
          )}
        </div>
      </div>
    </>
  );
};

export default RestrictionRules;
