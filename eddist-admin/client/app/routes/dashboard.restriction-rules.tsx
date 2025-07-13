import { useState } from "react";
import { useForm } from "react-hook-form";
import { FiPlus, FiEdit2, FiTrash2 } from "react-icons/fi";
import { 
  getUserRestrictionRules, 
  createUserRestrictionRule, 
  updateUserRestrictionRule, 
  deleteUserRestrictionRule 
} from "~/hooks/queries";
import { toast } from "react-toastify";
import type { components } from "~/openapi/schema";

type UserRestrictionRule = components["schemas"]["UserRestrictionRuleResponse"];
type CreateUserRestrictionRule = components["schemas"]["CreateUserRestrictionRuleRequest"];
type UpdateUserRestrictionRule = components["schemas"]["UpdateUserRestrictionRuleRequest"];

export default function UserRestrictionRules() {
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<UserRestrictionRule | null>(null);

  const { data: rules = [], refetch } = getUserRestrictionRules({});

  const handleCreateRule = async (rule: CreateUserRestrictionRule) => {
    try {
      const { mutate } = createUserRestrictionRule({ body: rule });
      await mutate();
      await refetch();
      setIsCreateModalOpen(false);
      toast.success("Rule created successfully");
    } catch (error) {
      toast.error("Failed to create rule");
    }
  };

  const handleUpdateRule = async (id: string, rule: UpdateUserRestrictionRule) => {
    try {
      const { mutate } = updateUserRestrictionRule({ 
        params: { path: { id } }, 
        body: rule 
      });
      await mutate();
      await refetch();
      setEditingRule(null);
      toast.success("Rule updated successfully");
    } catch (error) {
      toast.error("Failed to update rule");
    }
  };

  const handleDeleteRule = async (id: string) => {
    try {
      const { mutate } = deleteUserRestrictionRule({ 
        params: { path: { id } } 
      });
      await mutate();
      await refetch();
      toast.success("Rule deleted successfully");
    } catch (error) {
      toast.error("Failed to delete rule");
    }
  };

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-gray-900">User Restriction Rules</h1>
        <button
          onClick={() => setIsCreateModalOpen(true)}
          className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md flex items-center gap-2"
        >
          <FiPlus className="h-5 w-5" />
          Add Rule
        </button>
      </div>

      <div className="bg-white shadow-sm rounded-lg border border-gray-200">
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Name
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Filter Expression
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Type
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Created
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {rules.map((rule) => (
                <tr key={rule.id}>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="text-sm font-medium text-gray-900">{rule.name}</div>
                    {rule.description && (
                      <div className="text-sm text-gray-500">{rule.description}</div>
                    )}
                  </td>
                  <td className="px-6 py-4">
                    <code className="text-sm bg-gray-100 px-2 py-1 rounded font-mono">
                      {rule.filter_expression}
                    </code>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className="inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-gray-100 text-gray-800">
                      {rule.restriction_type.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase())}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full ${
                        rule.active
                          ? "bg-green-100 text-green-800"
                          : "bg-red-100 text-red-800"
                      }`}
                    >
                      {rule.active ? "Active" : "Inactive"}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                    {new Date(rule.created_at).toLocaleString()}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm font-medium">
                    <div className="flex gap-2">
                      <button
                        onClick={() => setEditingRule(rule)}
                        className="text-blue-600 hover:text-blue-900"
                      >
                        <FiEdit2 className="h-4 w-4" />
                      </button>
                      <button
                        onClick={() => {
                          if (confirm("Are you sure you want to delete this rule?")) {
                            handleDeleteRule(rule.id);
                          }
                        }}
                        className="text-red-600 hover:text-red-900"
                      >
                        <FiTrash2 className="h-4 w-4" />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Create Modal */}
      {isCreateModalOpen && (
        <CreateRuleModal
          onClose={() => setIsCreateModalOpen(false)}
          onSubmit={handleCreateRule}
          isLoading={false}
        />
      )}

      {/* Edit Modal */}
      {editingRule && (
        <EditRuleModal
          rule={editingRule}
          onClose={() => setEditingRule(null)}
          onSubmit={(rule) => handleUpdateRule(editingRule.id, rule)}
          isLoading={false}
        />
      )}
    </div>
  );
}

function CreateRuleModal({
  onClose,
  onSubmit,
  isLoading,
}: {
  onClose: () => void;
  onSubmit: (rule: CreateUserRestrictionRule) => void;
  isLoading: boolean;
}) {
  const { register, handleSubmit, formState: { errors } } = useForm<CreateUserRestrictionRule>();

  return (
    <div className="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
      <div className="relative top-20 mx-auto p-5 border w-full max-w-md shadow-lg rounded-md bg-white">
        <div className="mt-3">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Create Restriction Rule</h3>
          <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">Name</label>
              <input
                {...register("name", { required: "Name is required" })}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                placeholder="Rule name"
              />
              {errors.name && <p className="text-red-500 text-sm">{errors.name.message}</p>}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700">Restriction Type</label>
              <select
                {...register("restriction_type", { required: "Restriction type is required" })}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
              >
                <option value="">Select restriction type...</option>
                <option value="creating_response">Creating Response</option>
                <option value="creating_thread">Creating Thread</option>
                <option value="auth_code">Auth Code</option>
                <option value="all">All (Complete Ban)</option>
              </select>
              {errors.restriction_type && (
                <p className="text-red-500 text-sm">{errors.restriction_type.message}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700">Filter Expression</label>
              <textarea
                {...register("filter_expression", { required: "Filter expression is required" })}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                rows={3}
                placeholder='ip == 192.168.1.100 or ua contains "BadBot"'
              />
              {errors.filter_expression && (
                <p className="text-red-500 text-sm">{errors.filter_expression.message}</p>
              )}
              <p className="text-sm text-gray-500 mt-1">
                Examples: ip == 192.168.1.100, ua contains "spam", asn == 12345
              </p>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700">Description</label>
              <textarea
                {...register("description")}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                rows={2}
                placeholder="Optional description"
              />
            </div>

            <div className="flex items-center">
              <input
                {...register("active")}
                type="checkbox"
                className="h-4 w-4 text-blue-600 border-gray-300 rounded"
                defaultChecked
              />
              <label className="ml-2 block text-sm text-gray-900">Active</label>
            </div>

            <div className="flex gap-3 pt-4">
              <button
                type="submit"
                disabled={isLoading}
                className="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded-md disabled:opacity-50"
              >
                {isLoading ? "Creating..." : "Create"}
              </button>
              <button
                type="button"
                onClick={onClose}
                className="flex-1 bg-gray-300 hover:bg-gray-400 text-gray-700 py-2 px-4 rounded-md"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

function EditRuleModal({
  rule,
  onClose,
  onSubmit,
  isLoading,
}: {
  rule: UserRestrictionRule;
  onClose: () => void;
  onSubmit: (rule: UpdateUserRestrictionRule) => void;
  isLoading: boolean;
}) {
  const { register, handleSubmit, formState: { errors } } = useForm<UpdateUserRestrictionRule>({
    defaultValues: {
      name: rule.name,
      filter_expression: rule.filter_expression,
      restriction_type: rule.restriction_type,
      description: rule.description,
      active: rule.active,
    },
  });

  return (
    <div className="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
      <div className="relative top-20 mx-auto p-5 border w-full max-w-md shadow-lg rounded-md bg-white">
        <div className="mt-3">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Edit Restriction Rule</h3>
          <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">Name</label>
              <input
                {...register("name", { required: "Name is required" })}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                placeholder="Rule name"
              />
              {errors.name && <p className="text-red-500 text-sm">{errors.name.message}</p>}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700">Restriction Type</label>
              <select
                {...register("restriction_type", { required: "Restriction type is required" })}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
              >
                <option value="">Select restriction type...</option>
                <option value="creating_response">Creating Response</option>
                <option value="creating_thread">Creating Thread</option>
                <option value="auth_code">Auth Code</option>
                <option value="all">All (Complete Ban)</option>
              </select>
              {errors.restriction_type && (
                <p className="text-red-500 text-sm">{errors.restriction_type.message}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700">Filter Expression</label>
              <textarea
                {...register("filter_expression", { required: "Filter expression is required" })}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                rows={3}
                placeholder='ip == 192.168.1.100 or ua contains "BadBot"'
              />
              {errors.filter_expression && (
                <p className="text-red-500 text-sm">{errors.filter_expression.message}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700">Description</label>
              <textarea
                {...register("description")}
                className="mt-1 block w-full border border-gray-300 rounded-md px-3 py-2"
                rows={2}
                placeholder="Optional description"
              />
            </div>

            <div className="flex items-center">
              <input
                {...register("active")}
                type="checkbox"
                className="h-4 w-4 text-blue-600 border-gray-300 rounded"
              />
              <label className="ml-2 block text-sm text-gray-900">Active</label>
            </div>

            <div className="flex gap-3 pt-4">
              <button
                type="submit"
                disabled={isLoading}
                className="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded-md disabled:opacity-50"
              >
                {isLoading ? "Updating..." : "Update"}
              </button>
              <button
                type="button"
                onClick={onClose}
                className="flex-1 bg-gray-300 hover:bg-gray-400 text-gray-700 py-2 px-4 rounded-md"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}