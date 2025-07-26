import {
  Button,
  Dropdown,
  Label,
  Modal,
  Table,
  TextInput,
  Select,
  Checkbox,
} from "flowbite-react";
import { useState } from "react";
import { BiDotsHorizontalRounded } from "react-icons/bi";
import { FaPlus } from "react-icons/fa";
import { Controller, useForm } from "react-hook-form";
import {
  deleteRestrictionRule,
  getRestrictionRules,
  updateRestrictionRule,
  createRestrictionRule,
} from "~/hooks/queries";
import { toast } from "react-toastify";

interface RestrictionRule {
  id: string;
  name: string;
  rule_type: "ASN" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string | null;
  created_at: string;
  updated_at: string;
  created_by_email: string;
}

interface CreateRestrictionRuleForm {
  name: string;
  rule_type: "ASN" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string;
}

interface EditRestrictionRuleForm {
  id: string;
  name: string;
  rule_type: "ASN" | "IP" | "IPCidr" | "UserAgent";
  rule_value: string;
  expires_at?: string;
}

const RestrictionRules = () => {
  const { data: restrictionRules, refetch } = getRestrictionRules({});
  const [openCreateModal, setOpenCreateModal] = useState(false);
  const [openEditModal, setOpenEditModal] = useState(false);
  const [selectedRule, setSelectedRule] = useState<RestrictionRule | undefined>(
    undefined
  );
  const [createNeverExpires, setCreateNeverExpires] = useState(true);
  const [editNeverExpires, setEditNeverExpires] = useState(true);
  const {
    register: registerCreate,
    handleSubmit: handleSubmitCreate,
    control: controlCreate,
    reset: resetCreate,
  } = useForm<CreateRestrictionRuleForm>();
  const {
    register: registerEdit,
    handleSubmit: handleSubmitEdit,
    control: controlEdit,
    reset: resetEdit,
  } = useForm<EditRestrictionRuleForm>();

  const ruleTypeOptions = [
    { value: "ASN", label: "ASN" },
    { value: "IP", label: "IP Address" },
    { value: "IPCidr", label: "IP CIDR" },
    { value: "UserAgent", label: "User Agent" },
  ];

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

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
        show={openCreateModal}
        onClose={() => {
          resetCreate();
          setCreateNeverExpires(true);
          setOpenCreateModal(false);
        }}
      >
        <Modal.Header>Create Restriction Rule</Modal.Header>
        <Modal.Body>
          <form
            onSubmit={handleSubmitCreate(async (data) => {
              try {
                const { mutate } = createRestrictionRule({
                  body: {
                    name: data.name,
                    rule_type: data.rule_type,
                    rule_value: data.rule_value,
                    expires_at: createNeverExpires || !data.expires_at
                      ? undefined
                      : new Date(data.expires_at).toISOString(),
                  },
                });
                await mutate();
                setOpenCreateModal(false);
                resetCreate();
                setCreateNeverExpires(true);
                toast.success("Successfully created restriction rule");
                await refetch();
              } catch (e) {
                toast.error("Failed to create restriction rule");
              }
            })}
          >
            <div className="flex flex-col space-y-4">
              <div>
                <Label>Name</Label>
                <TextInput
                  placeholder="Rule name..."
                  required
                  {...registerCreate("name", { required: true })}
                />
              </div>
              <div>
                <Label>Rule Type</Label>
                <Controller
                  name="rule_type"
                  control={controlCreate}
                  rules={{ required: true }}
                  render={({ field }) => (
                    <Select
                      required
                      value={field.value}
                      onChange={(e) => field.onChange(e.target.value)}
                    >
                      <option value="">Select rule type...</option>
                      {ruleTypeOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </Select>
                  )}
                />
              </div>
              <div>
                <Label>Rule Value</Label>
                <TextInput
                  placeholder="Rule value..."
                  required
                  {...registerCreate("rule_value", { required: true })}
                />
              </div>
              <div>
                <div className="flex items-center space-x-2 mb-3">
                  <Checkbox
                    id="create-never-expires"
                    checked={createNeverExpires}
                    onChange={(e) => setCreateNeverExpires(e.target.checked)}
                  />
                  <Label htmlFor="create-never-expires">Never expires</Label>
                </div>
                {!createNeverExpires && (
                  <div>
                    <Label>Expires At</Label>
                    <TextInput
                      type="datetime-local"
                      {...registerCreate("expires_at")}
                    />
                  </div>
                )}
              </div>
            </div>
            <Button type="submit" className="mt-4">
              Create Rule
            </Button>
          </form>
        </Modal.Body>
      </Modal>

      <Modal
        show={openEditModal}
        onClose={() => {
          resetEdit();
          setEditNeverExpires(true);
          setOpenEditModal(false);
        }}
      >
        <Modal.Header>Edit Restriction Rule</Modal.Header>
        <Modal.Body>
          <form
            onSubmit={handleSubmitEdit(async (data) => {
              try {
                const { mutate } = updateRestrictionRule({
                  params: {
                    path: {
                      rule_id: selectedRule!.id,
                    },
                  },
                  body: {
                    name: data.name,
                    rule_type: data.rule_type,
                    rule_value: data.rule_value,
                    expires_at: editNeverExpires || !data.expires_at
                      ? undefined
                      : new Date(data.expires_at).toISOString(),
                  },
                });
                await mutate();
                setOpenEditModal(false);
                resetEdit();
                setEditNeverExpires(true);
                toast.success("Successfully updated restriction rule");
                await refetch();
              } catch (e) {
                toast.error("Failed to update restriction rule");
              }
            })}
          >
            <div className="flex flex-col space-y-4">
              <input
                type="hidden"
                {...registerEdit("id")}
                value={selectedRule?.id}
              />
              <div>
                <Label>Name</Label>
                <TextInput
                  placeholder="Rule name..."
                  required
                  defaultValue={selectedRule?.name}
                  {...registerEdit("name", { required: true })}
                />
              </div>
              <div>
                <Label>Rule Type</Label>
                <Controller
                  name="rule_type"
                  control={controlEdit}
                  rules={{ required: true }}
                  defaultValue={selectedRule?.rule_type}
                  render={({ field }) => (
                    <Select
                      required
                      value={field.value}
                      onChange={(e) => field.onChange(e.target.value)}
                    >
                      <option value="">Select rule type...</option>
                      {ruleTypeOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </Select>
                  )}
                />
              </div>
              <div>
                <Label>Rule Value</Label>
                <TextInput
                  placeholder="Rule value..."
                  required
                  defaultValue={selectedRule?.rule_value}
                  {...registerEdit("rule_value", { required: true })}
                />
              </div>
              <div>
                <div className="flex items-center space-x-2 mb-3">
                  <Checkbox
                    id="edit-never-expires"
                    checked={editNeverExpires}
                    onChange={(e) => setEditNeverExpires(e.target.checked)}
                  />
                  <Label htmlFor="edit-never-expires">Never expires</Label>
                </div>
                {!editNeverExpires && (
                  <div>
                    <Label>Expires At</Label>
                    <TextInput
                      type="datetime-local"
                      defaultValue={
                        selectedRule?.expires_at
                          ? new Date(selectedRule.expires_at)
                              .toISOString()
                              .slice(0, 16)
                          : ""
                      }
                      {...registerEdit("expires_at")}
                    />
                  </div>
                )}
              </div>
            </div>
            <Button type="submit" className="mt-4">
              Update Rule
            </Button>
          </form>
        </Modal.Body>
      </Modal>

      <div className="p-2 lg:p-8">
        <div className="flex">
          <h1 className="text-3xl font-bold flex-grow">Restriction Rules</h1>
          <button
            className="mr-2 bg-slate-400 p-4 rounded-xl shadow-lg hover:bg-slate-500"
            onClick={() => setOpenCreateModal(true)}
          >
            <FaPlus />
          </button>
        </div>
        <Table className="mt-4">
          <Table.Head>
            <Table.HeadCell>Name</Table.HeadCell>
            <Table.HeadCell>Type</Table.HeadCell>
            <Table.HeadCell>Value</Table.HeadCell>
            <Table.HeadCell>Expires</Table.HeadCell>
            <Table.HeadCell>Created By</Table.HeadCell>
            <Table.HeadCell>Created At</Table.HeadCell>
            <Table.HeadCell></Table.HeadCell>
          </Table.Head>
          <Table.Body className="divide-y">
            {restrictionRules?.map((rule) => (
              <Table.Row key={rule.id}>
                <Table.Cell className="font-medium">{rule.name}</Table.Cell>
                <Table.Cell>
                  <span className="px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800">
                    {rule.rule_type}
                  </span>
                </Table.Cell>
                <Table.Cell className="font-mono text-sm">
                  {rule.rule_value}
                </Table.Cell>
                <Table.Cell>
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
                </Table.Cell>
                <Table.Cell>{rule.created_by_email}</Table.Cell>
                <Table.Cell>{formatDate(rule.created_at)}</Table.Cell>
                <Table.Cell>
                  <div className="text-right">
                    <Dropdown label={<BiDotsHorizontalRounded />}>
                      <Dropdown.Item
                        onClick={() => {
                          setOpenEditModal(true);
                          setSelectedRule(rule);
                          setEditNeverExpires(!rule.expires_at);
                        }}
                      >
                        Edit
                      </Dropdown.Item>
                      <Dropdown.Item
                        className="text-red-500"
                        onClick={async () => {
                          try {
                            const { mutate } = deleteRestrictionRule({
                              params: {
                                path: {
                                  rule_id: rule.id,
                                },
                              },
                            });
                            await mutate();
                            toast.success(
                              "Successfully deleted restriction rule"
                            );
                            await refetch();
                          } catch (e) {
                            toast.error("Failed to delete restriction rule");
                          }
                        }}
                      >
                        Delete
                      </Dropdown.Item>
                    </Dropdown>
                  </div>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      </div>
    </>
  );
};

export default RestrictionRules;
