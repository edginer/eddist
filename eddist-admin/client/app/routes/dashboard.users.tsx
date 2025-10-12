import { Alert, Button, Label, Table, TextInput, Modal } from "flowbite-react";
import { useCallback, useState } from "react";
import client from "~/openapi/client";
import { components } from "~/openapi/schema";
import { Link } from "react-router";

const UserSearchPage = () => {
  const [userId, setUserId] = useState("");
  const [userName, setUserName] = useState("");
  const [authedToken, setAuthedToken] = useState("");
  const [searchError, setSearchError] = useState("");
  const [actionMessage, setActionMessage] = useState("");
  const [userData, setUserData] = useState<components["schemas"]["User"]>();
  const [userIdpBindings, setUserIdpBindings] = useState<
    components["schemas"]["UserIdpBinding"][]
  >([]);
  const [isUpdating, setIsUpdating] = useState(false);
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [newEnabledStatus, setNewEnabledStatus] = useState(false);

  const handleSearch = useCallback(async () => {
    // Check if at least one search field is filled
    if (!userId && !userName && !authedToken) {
      setSearchError("At least one search field is required.");
      return;
    }

    try {
      const { data } = await client.GET("/users/search/", {
        params: {
          query: {
            user_id: userId || undefined,
            user_name: userName || undefined,
            authed_token: authedToken || undefined,
          },
        },
      });

      setSearchError("");
      setActionMessage("");

      if (data == null) {
        setSearchError("No user found.");
        return;
      }

      if (data?.length > 1) {
        setSearchError("Multiple users found. Showing the first one.");
      }

      setUserData(data[0]);
      setUserIdpBindings(data[0].idp_bindings || []);
    } catch (error: any) {
      setSearchError(error.message);
      setUserData(undefined);
      setUserIdpBindings([]);
    }
  }, [userId, userName, authedToken]);

  const handleToggleEnabledRequest = () => {
    if (!userData) return;

    // Set the new status we want to apply
    const newStatus = !userData.enabled;
    setNewEnabledStatus(newStatus);

    // Open the confirmation modal
    setShowConfirmModal(true);
  };

  const handleToggleEnabled = async () => {
    if (!userData || !userData.id) {
      setActionMessage("No user selected to update");
      return;
    }

    setIsUpdating(true);
    try {
      const { data } = await client.PATCH(`/users/{user_id}/status/`, {
        body: {
          enabled: newEnabledStatus,
        },
        params: {
          path: {
            user_id: userData.id,
          },
        },
      });

      if (!data) {
        setActionMessage("Failed to update user status");
        return;
      }

      setUserData({
        ...userData,
        enabled: data.enabled,
      });

      setActionMessage(
        `User ${newEnabledStatus ? "enabled" : "disabled"} successfully`
      );
    } catch (error: any) {
      setActionMessage(`Error: ${error.message}`);
    } finally {
      setIsUpdating(false);
      setShowConfirmModal(false);
    }
  };

  return (
    <div className="p-4">
      <div className="flex">
        <h1 className="text-3xl font-bold flex-grow">User Search</h1>
      </div>
      <div className="flex flex-col p-2 sm:p-8 m-4 xl:m-8 h-[calc(100vh-140px)] items-center">
        <div className="flex w-full flex-col xl:flex-row mb-8 xl:items-end">
          <div className="flex flex-1 flex-col">
            <div className="mb-4">
              <Label htmlFor="user-id-input" className="block mb-2">
                User ID (UUID)
              </Label>
              <TextInput
                id="user-id-input"
                className="w-full"
                value={userId}
                onChange={(e) => setUserId(e.target.value)}
                placeholder="Enter User ID"
              />
            </div>

            <div className="mb-4">
              <Label htmlFor="user-name-input" className="block mb-2">
                Username
              </Label>
              <TextInput
                id="user-name-input"
                className="w-full"
                value={userName}
                onChange={(e) => setUserName(e.target.value)}
                placeholder="Enter Username"
              />
            </div>

            <div>
              <Label htmlFor="authed-token-input" className="block mb-2">
                Authed Token (UUID)
              </Label>
              <TextInput
                id="authed-token-input"
                className="w-full"
                value={authedToken}
                onChange={(e) => setAuthedToken(e.target.value)}
                placeholder="Enter Authed Token"
              />
            </div>
          </div>

          <Button
            onClick={handleSearch}
            type="submit"
            className="mt-2 xl:h-12 px-8 xl:mx-4"
          >
            Search
          </Button>
        </div>

        <div className="w-full">
          {searchError && (
            <Alert color="failure" className="mb-4">
              {searchError}
            </Alert>
          )}

          {actionMessage && (
            <Alert
              color={actionMessage.startsWith("Error") ? "failure" : "success"}
              className="mb-4"
            >
              {actionMessage}
            </Alert>
          )}

          {userData && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-2">User Information</h2>
                <Table>
                  <Table.Body className="divide-y">
                    <Table.Row>
                      <Table.Cell className="font-medium">User ID</Table.Cell>
                      <Table.Cell>{userData?.id ?? "N/A"}</Table.Cell>
                    </Table.Row>
                    <Table.Row>
                      <Table.Cell className="font-medium">Username</Table.Cell>
                      <Table.Cell>{userData?.user_name ?? "N/A"}</Table.Cell>
                    </Table.Row>
                    <Table.Row>
                      <Table.Cell className="font-medium">Enabled</Table.Cell>
                      <Table.Cell className="flex items-center gap-3">
                        <span className="mr-2">
                          {userData?.enabled === true ? "Yes" : "No"}
                        </span>
                        <div className="flex items-center">
                          <label className="relative inline-flex items-center cursor-pointer">
                            <input
                              type="checkbox"
                              className="sr-only peer"
                              checked={userData?.enabled === true}
                              onChange={handleToggleEnabledRequest}
                              disabled={isUpdating}
                            />
                            <div className="w-11 h-6 bg-gray-200 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                          </label>
                        </div>
                      </Table.Cell>
                    </Table.Row>
                  </Table.Body>
                </Table>
              </div>

              {userIdpBindings.length > 0 && (
                <div>
                  <h2 className="text-xl font-semibold mb-2">
                    Identity Provider Bindings
                  </h2>
                  <Table>
                    <Table.Head>
                      <Table.HeadCell>IDP Name</Table.HeadCell>
                      <Table.HeadCell>IDP Sub</Table.HeadCell>
                    </Table.Head>
                    <Table.Body className="divide-y">
                      {userIdpBindings.map((binding) => (
                        <Table.Row key={binding.id}>
                          <Table.Cell>{binding.idp_name}</Table.Cell>
                          <Table.Cell>{binding.idp_sub}</Table.Cell>
                        </Table.Row>
                      ))}
                    </Table.Body>
                  </Table>
                </div>
              )}

              {userData.authed_token_ids &&
                userData.authed_token_ids.length > 0 && (
                  <div>
                    <h2 className="text-xl font-semibold mb-2">
                      Authed Tokens
                    </h2>
                    <Table>
                      <Table.Head>
                        <Table.HeadCell>Token ID</Table.HeadCell>
                        <Table.HeadCell>Actions</Table.HeadCell>
                      </Table.Head>
                      <Table.Body className="divide-y">
                        {userData.authed_token_ids.map((token) => (
                          <Table.Row key={token}>
                            <Table.Cell>{token}</Table.Cell>
                            <Table.Cell>
                              <Link
                                to={`/dashboard/authed-token/?token=${token}`}
                                className="text-blue-600 hover:underline"
                              >
                                View Details
                              </Link>
                            </Table.Cell>
                          </Table.Row>
                        ))}
                      </Table.Body>
                    </Table>
                  </div>
                )}
            </div>
          )}
        </div>
      </div>

      {/* Confirmation Modal */}
      <Modal show={showConfirmModal} onClose={() => setShowConfirmModal(false)}>
        <Modal.Header>
          {newEnabledStatus ? "Enable User" : "Disable User"}
        </Modal.Header>
        <Modal.Body>
          {newEnabledStatus ? (
            <p>
              Enabling this user will re-enable all authed tokens associated
              with the user.
            </p>
          ) : (
            <p>
              Disabling this user will revoke all authed tokens associated with
              the user.
            </p>
          )}
        </Modal.Body>
        <Modal.Footer>
          <Button onClick={handleToggleEnabled} disabled={isUpdating}>
            {isUpdating ? "Processing..." : "Confirm"}
          </Button>
          <Button color="gray" onClick={() => setShowConfirmModal(false)}>
            Cancel
          </Button>
        </Modal.Footer>
      </Modal>
    </div>
  );
};

export default UserSearchPage;
