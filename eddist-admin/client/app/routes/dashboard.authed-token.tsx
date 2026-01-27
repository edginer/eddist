import { useSearchParams } from "react-router";
import { Alert, Badge, Button, Card, Label, TextInput } from "flowbite-react";
import { useCallback, useEffect, useState } from "react";
import client from "~/openapi/client";
import { components } from "~/openapi/schema";
import { useDeleteAuthedToken } from "~/hooks/deleteAuthedToken";

const Page = () => {
  const [searchParams] = useSearchParams();

  const [authedToken, setAuthedToken] = useState("");
  const [authedTokenData, setAuthedTokenData] =
    useState<components["schemas"]["AuthedToken"]>();
  const [searchError, setSearchError] = useState("");
  const [showAdditionalInfo, setShowAdditionalInfo] = useState(false);
  const deleteAuthedToken = useDeleteAuthedToken();

  const handleAuthedTokenSearch = useCallback(async () => {
    if (!authedToken) {
      setSearchError("Authed Token ID is required.");
      return;
    }

    try {
      const { data } = await client.GET(`/authed_tokens/{authed_token_id}/`, {
        params: {
          path: { authed_token_id: authedToken },
        },
      });

      setSearchError("");
      setAuthedTokenData(data);
    } catch (error: unknown) {
      if (error instanceof Error) {
        setSearchError(error.message);
      } else {
        setSearchError("An unknown error occurred");
      }
    }
  }, [authedToken, setAuthedTokenData]);

  const handleRevokeToken = useCallback(async () => {
    if (!authedTokenData?.id) return;
    if (!confirm("Are you sure you want to revoke this token?")) return;
    await deleteAuthedToken(authedTokenData.id, false);
    await handleAuthedTokenSearch();
  }, [authedTokenData?.id, deleteAuthedToken, handleAuthedTokenSearch]);

  const handleRevokeAllFromOriginIp = useCallback(async () => {
    if (!authedTokenData?.id) return;
    if (
      !confirm(
        `Are you sure you want to revoke ALL tokens from origin IP: ${authedTokenData.origin_ip}?`,
      )
    )
      return;
    await deleteAuthedToken(authedTokenData.id, true);
    await handleAuthedTokenSearch();
  }, [
    authedTokenData?.id,
    authedTokenData?.origin_ip,
    deleteAuthedToken,
    handleAuthedTokenSearch,
  ]);

  useEffect(() => {
    if (searchParams.has("token")) {
      const token = searchParams.get("token");
      if (token) {
        setAuthedToken(token);
      }
    }
  }, [searchParams]);

  const formatDateTime = (dateStr: string | null | undefined) => {
    if (!dateStr) return "N/A";
    return new Date(dateStr).toLocaleString();
  };

  return (
    <div className="p-4">
      <h1 className="text-3xl font-bold mb-6">Authed Token</h1>

      {/* Search Section */}
      <Card className="mb-6">
        <div className="flex flex-col sm:flex-row gap-4 items-end">
          <div className="flex-1">
            <Label htmlFor="search-authed-token-input" className="mb-2 block">
              Search by Token ID
            </Label>
            <TextInput
              id="search-authed-token-input"
              placeholder="Enter Authed Token ID..."
              value={authedToken}
              onChange={(e) => setAuthedToken(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAuthedTokenSearch()}
            />
          </div>
          <Button onClick={handleAuthedTokenSearch}>Search</Button>
        </div>
        {searchError && (
          <Alert color="failure" className="mt-4">
            {searchError}
          </Alert>
        )}
      </Card>

      {/* Result Section */}
      {authedTokenData && (
        <div className="space-y-6">
          {/* Header Card */}
          <Card>
            <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
              <div>
                <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
                  Token Details
                </h2>
                <p className="text-sm text-gray-500 dark:text-gray-400 font-mono mt-1">
                  {authedTokenData.id}
                </p>
              </div>
              <Badge
                color={authedTokenData.validity ? "success" : "failure"}
                size="lg"
              >
                {authedTokenData.validity ? "Valid" : "Revoked"}
              </Badge>
            </div>
          </Card>

          {/* Details Grid */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Token Information */}
            <Card>
              <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
                Token Information
              </h3>
              <dl className="space-y-3">
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Token
                  </dt>
                  <dd className="font-mono text-sm break-all">
                    {authedTokenData.token}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Created At
                  </dt>
                  <dd>{formatDateTime(authedTokenData.created_at)}</dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Authed At
                  </dt>
                  <dd>{formatDateTime(authedTokenData.authed_at)}</dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Last Wrote At
                  </dt>
                  <dd>{formatDateTime(authedTokenData.last_wrote_at)}</dd>
                </div>
              </dl>
            </Card>

            {/* Network Information */}
            <Card>
              <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
                Network Information
              </h3>
              <dl className="space-y-3">
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Origin IP
                  </dt>
                  <dd className="font-mono">{authedTokenData.origin_ip}</dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Reduced Origin IP
                  </dt>
                  <dd className="font-mono">
                    {authedTokenData.reduced_origin_ip}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    ASN Number
                  </dt>
                  <dd className="font-mono">
                    {authedTokenData.asn_num ?? "N/A"}
                  </dd>
                </div>
              </dl>
            </Card>

            {/* User Agent Information */}
            <Card>
              <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
                User Agent Information
              </h3>
              <dl className="space-y-3">
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Writing UA
                  </dt>
                  <dd className="text-sm break-all">
                    {authedTokenData.writing_ua}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500 dark:text-gray-400">
                    Authed UA
                  </dt>
                  <dd className="text-sm break-all">
                    {authedTokenData.authed_ua ?? "N/A"}
                  </dd>
                </div>
              </dl>
            </Card>

            {/* Additional Info */}
            <Card>
              <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
                Additional Information
              </h3>
              {authedTokenData.additional_info ? (
                <div>
                  <button
                    onClick={() => setShowAdditionalInfo(!showAdditionalInfo)}
                    className="flex items-center text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                  >
                    <svg
                      className={`w-4 h-4 mr-1 transition-transform ${
                        showAdditionalInfo ? "rotate-90" : ""
                      }`}
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 5l7 7-7 7"
                      />
                    </svg>
                    {showAdditionalInfo ? "Hide" : "Show"} JSON Data
                  </button>
                  {showAdditionalInfo && (
                    <pre className="mt-3 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg text-sm overflow-auto max-h-64">
                      {JSON.stringify(authedTokenData.additional_info, null, 2)}
                    </pre>
                  )}
                </div>
              ) : (
                <p className="text-gray-500 dark:text-gray-400">
                  No additional information available
                </p>
              )}
            </Card>
          </div>

          {/* Actions */}
          <Card>
            <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
              Actions
            </h3>
            <div className="flex flex-wrap gap-3">
              <Button
                color="failure"
                onClick={handleRevokeToken}
                disabled={authedTokenData.validity === false}
              >
                Revoke This Token
              </Button>
              <Button
                color="failure"
                onClick={handleRevokeAllFromOriginIp}
                disabled={authedTokenData.validity === false}
              >
                Revoke All Tokens from Origin IP
              </Button>
            </div>
            {authedTokenData.validity === false && (
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-3">
                This token has already been revoked.
              </p>
            )}
          </Card>
        </div>
      )}
    </div>
  );
};

export default Page;
