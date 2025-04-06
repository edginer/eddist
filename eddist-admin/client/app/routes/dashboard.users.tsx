import { Alert, Button, Label, Table, TextInput } from "flowbite-react";
import { useCallback, useState } from "react";
import client from "~/openapi/client";
import { components } from "~/openapi/schema";

const UserSearchPage = () => {
  const [userId, setUserId] = useState("");
  const [userName, setUserName] = useState("");
  const [authedToken, setAuthedToken] = useState("");
  const [searchError, setSearchError] = useState("");
  const [userData, setUserData] = useState<components["schemas"]["User"]>();
  const [userIdpBindings, setUserIdpBindings] = useState<
    components["schemas"]["UserIdpBinding"][]
  >([]);

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

  return (
    <div className="p-4">
      <div className="flex">
        <h1 className="text-3xl font-bold flex-grow">User Search</h1>
      </div>
      <div className="flex flex-col p-2 sm:p-8 md:border m-4 xl:m-8 border-gray-700 h-[calc(100vh-140px)] items-center">
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
                      <Table.Cell>
                        {userData?.enabled === true ? "Yes" : "No"}
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
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default UserSearchPage;
