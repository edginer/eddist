import { useSearchParams } from "react-router";
import { Alert, Button, Label, Table, TextInput } from "flowbite-react";
import { useCallback, useEffect, useState } from "react";
import client from "~/openapi/client";
import { components } from "~/openapi/schema";

const Page = () => {
  const [searchParams] = useSearchParams();

  const [authedToken, setAuthedToken] = useState("");
  const [authedTokenData, setAuthedTokenData] =
    useState<components["schemas"]["AuthedToken"]>();
  const [setsearchError, setSetsearchError] = useState("");

  const handleAuthedTokenSearch = useCallback(async () => {
    if (!authedToken) {
      setSetsearchError("Authed Token ID is required.");
      return;
    }

    try {
      const { data } = await client.GET(`/authed_tokens/{authed_token_id}/`, {
        params: {
          path: { authed_token_id: authedToken },
        },
      });

      setSetsearchError("");
      setAuthedTokenData(data);
    } catch (error: any) {
      setSetsearchError(error.message);
    }
  }, [authedToken, setAuthedTokenData]);

  useEffect(() => {
    if (searchParams.has("token")) {
      const token = searchParams.get("token");
      if (token) {
        setAuthedToken(token);
      }
    }
  }, [searchParams]);

  return (
    <div className="p-4">
      <div className="flex">
        <h1 className="text-3xl font-bold grow">Authed Token</h1>
      </div>
      <div className="flex flex-col items-center p-2 sm:p-8 md:border m-4 xl:m-8 border-gray-700 h-[calc(100vh-140px)]">
        <Label htmlFor="search-authed-token-input" className="pb-2">
          Search Authed Token
        </Label>
        <div className="flex flex-row">
          <TextInput
            id="search-authed-token-input"
            className="md:w-72 lg:w-96"
            value={authedToken}
            onChange={(e) => setAuthedToken(e.target.value)}
          />
          <Button
            className="ml-2"
            onClick={handleAuthedTokenSearch}
            type="submit"
          >
            Search
          </Button>
        </div>
        <div className="p-4">
          {setsearchError && <Alert color="failure">{setsearchError}</Alert>}
          {authedTokenData && (
            <div className="p-2">
              <Table>
                <Table.Body className="divide-y">
                  <Table.Row className="">
                    <Table.Cell>Authed Token ID</Table.Cell>
                    <Table.Cell>{authedTokenData?.id ?? "N/A"}</Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Token</Table.Cell>
                    <Table.Cell>{authedTokenData?.token ?? "N/A"}</Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Origin IP</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.origin_ip ?? "N/A"}
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Writing UA</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.writing_ua ?? "N/A"}
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Authed UA</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.authed_ua ?? "N/A"}
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Created At</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.created_at ?? "N/A"}
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Authed At</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.authed_at ?? "N/A"}
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Validaity</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.validity === true ? "true" : "false"}
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Last wrote at</Table.Cell>
                    <Table.Cell>
                      {authedTokenData?.last_wrote_at ?? "N/A"}
                    </Table.Cell>
                  </Table.Row>
                </Table.Body>
              </Table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default Page;
