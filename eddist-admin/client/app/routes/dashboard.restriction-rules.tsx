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
import {
  getRestrictionRules,
  useCreateRestrictionRule,
  useUpdateRestrictionRule,
  useDeleteRestrictionRule,
} from "~/hooks/queries";
import { formatDateTime } from "~/utils/format";
import { useCrudModalState } from "~/hooks/useCrudModalState";
import RestrictionRuleForm from "~/components/RestrictionRuleForm";

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
      <Modal
        show={modal.isCreateOpen}
        onClose={() => modal.closeCreate()}
      >
        <ModalHeader className="border-gray-200">
          Create Restriction Rule
        </ModalHeader>
        <ModalBody>
          <RestrictionRuleForm
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

      {modal.editingItem && (
        <Modal
          show={modal.isEditOpen}
          onClose={() => modal.closeEdit()}
        >
          <ModalHeader className="border-gray-200">
            Edit Restriction Rule
          </ModalHeader>
          <ModalBody>
            <RestrictionRuleForm
              mode="edit"
              defaultValues={modal.editingItem}
              onSubmit={(data) => {
                updateMutation.mutate(
                  {
                    params: { path: { rule_id: modal.editingItem!.id } },
                    body: data,
                  },
                  { onSuccess: () => modal.closeEdit() },
                );
              }}
            />
          </ModalBody>
        </Modal>
      )}

      <div className="p-2 lg:p-8">
        <div className="flex">
          <h1 className="text-3xl font-bold grow">Restriction Rules</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => modal.openCreate()}
          >
            <FaPlus />
          </button>
        </div>
        <Table className="mt-4">
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
                  <span className="px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800">
                    {rule.rule_type}
                  </span>
                </TableCell>
                <TableCell className="font-mono text-sm">
                  {rule.rule_value}
                </TableCell>
                <TableCell>
                  <span
                    className={`px-2 py-1 text-xs font-semibold rounded-full ${
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
                      <DropdownItem
                        onClick={() => modal.openEdit(rule)}
                      >
                        Edit
                      </DropdownItem>
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
    </>
  );
};

export default RestrictionRules;
