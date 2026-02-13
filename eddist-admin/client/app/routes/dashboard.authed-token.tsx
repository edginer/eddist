import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router";
import {
  Alert,
  Badge,
  Button,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
  TextInput,
} from "flowbite-react";
import {
  type ColumnDef,
  type SortingState,
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useQueryClient } from "@tanstack/react-query";
import { listAuthedTokens } from "~/hooks/queries";
import { useDeleteAuthedToken } from "~/hooks/deleteAuthedToken";
import client from "~/openapi/client";
import type { components } from "~/openapi/schema";

type AuthedToken = components["schemas"]["AuthedToken"];

const formatDateTime = (dateStr: string | null | undefined) => {
  if (!dateStr) return "N/A";
  return new Date(dateStr).toLocaleString();
};

const Page = () => {
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const deleteAuthedToken = useDeleteAuthedToken();

  // Open detail modal when navigating with ?token=<id> from responses page
  useEffect(() => {
    const tokenId = searchParams.get("token");
    if (!tokenId) return;
    (async () => {
      const { data } = await client.GET("/authed_tokens/{authed_token_id}/", {
        params: { path: { authed_token_id: tokenId } },
      });
      if (data) setSelectedToken(data);
    })();
  }, [searchParams]);

  // Single token lookup
  const [tokenIdSearch, setTokenIdSearch] = useState("");
  const [tokenSearchError, setTokenSearchError] = useState("");

  const handleTokenIdSearch = useCallback(async () => {
    if (!tokenIdSearch) {
      setTokenSearchError("Token ID is required.");
      return;
    }
    try {
      const { data } = await client.GET("/authed_tokens/{authed_token_id}/", {
        params: { path: { authed_token_id: tokenIdSearch } },
      });
      setTokenSearchError("");
      if (data) setSelectedToken(data);
    } catch {
      setTokenSearchError("Failed to fetch token.");
    }
  }, [tokenIdSearch]);

  // Filter state
  const [originIpFilter, setOriginIpFilter] = useState("");
  const [writingUaFilter, setWritingUaFilter] = useState("");
  const [authedUaFilter, setAuthedUaFilter] = useState("");
  const [asnNumFilter, setAsnNumFilter] = useState("");
  const [validityFilter, setValidityFilter] = useState<string>("");

  // Applied filters (only sent to API on Search click)
  const [appliedFilters, setAppliedFilters] = useState<{
    origin_ip?: string;
    writing_ua?: string;
    authed_ua?: string;
    asn_num?: number;
    validity?: boolean;
  }>({});

  // Pagination & sorting
  const [page, setPage] = useState(1);
  const [perPage] = useState(50);
  const [sorting, setSorting] = useState<SortingState>([
    { id: "created_at", desc: true },
  ]);

  // Detail modal
  const [selectedToken, setSelectedToken] = useState<AuthedToken | null>(null);
  const [showAdditionalInfo, setShowAdditionalInfo] = useState(false);

  const queryParams = useMemo(
    () => ({
      page,
      per_page: perPage,
      ...appliedFilters,
      ...(sorting.length > 0
        ? {
            sort_by: sorting[0].id,
            sort_order: sorting[0].desc ? "desc" : "asc",
          }
        : {}),
    }),
    [page, perPage, appliedFilters, sorting],
  );

  const { data, isLoading, isError } = listAuthedTokens(queryParams);

  const handleSearch = useCallback(() => {
    const filters: typeof appliedFilters = {};
    if (originIpFilter) filters.origin_ip = originIpFilter;
    if (writingUaFilter) filters.writing_ua = writingUaFilter;
    if (authedUaFilter) filters.authed_ua = authedUaFilter;
    if (asnNumFilter) filters.asn_num = Number(asnNumFilter);
    if (validityFilter !== "") filters.validity = validityFilter === "true";
    setAppliedFilters(filters);
    setPage(1);
  }, [
    originIpFilter,
    writingUaFilter,
    authedUaFilter,
    asnNumFilter,
    validityFilter,
  ]);

  const handleRevokeToken = useCallback(async () => {
    if (!selectedToken?.id) return;
    if (!confirm("Are you sure you want to revoke this token?")) return;
    await deleteAuthedToken(selectedToken.id, false);
    queryClient.invalidateQueries({ queryKey: ["/authed_tokens"] });
    setSelectedToken(null);
  }, [selectedToken?.id, deleteAuthedToken, queryClient]);

  const handleRevokeAllFromOriginIp = useCallback(async () => {
    if (!selectedToken?.id) return;
    if (
      !confirm(
        `Are you sure you want to revoke ALL tokens from origin IP: ${selectedToken.origin_ip}?`,
      )
    )
      return;
    await deleteAuthedToken(selectedToken.id, true);
    queryClient.invalidateQueries({ queryKey: ["/authed_tokens"] });
    setSelectedToken(null);
  }, [
    selectedToken?.id,
    selectedToken?.origin_ip,
    deleteAuthedToken,
    queryClient,
  ]);

  const columns = useMemo<ColumnDef<AuthedToken>[]>(
    () => [
      {
        accessorKey: "id",
        header: "ID",
        size: 120,
        enableSorting: false,
        cell: (info) => (
          <span className="font-mono text-xs">
            {(info.getValue() as string).slice(0, 8)}...
          </span>
        ),
      },
      {
        accessorKey: "origin_ip",
        header: "Origin IP",
        size: 140,
        enableSorting: false,
        cell: (info) => (
          <span className="font-mono text-xs">{info.getValue() as string}</span>
        ),
      },
      {
        accessorKey: "asn_num",
        header: "ASN",
        size: 80,
        enableSorting: false,
      },
      {
        accessorKey: "validity",
        header: "Status",
        size: 90,
        enableSorting: false,
        cell: (info) => (
          <Badge color={info.getValue() ? "success" : "failure"} size="xs">
            {info.getValue() ? "Valid" : "Revoked"}
          </Badge>
        ),
      },
      {
        accessorKey: "created_at",
        header: "Created",
        size: 170,
        cell: (info) => formatDateTime(info.getValue() as string),
      },
      {
        accessorKey: "authed_at",
        header: "Authed",
        size: 170,
        cell: (info) => formatDateTime(info.getValue() as string | null),
      },
      {
        accessorKey: "last_wrote_at",
        header: "Last Wrote",
        size: 170,
        cell: (info) => formatDateTime(info.getValue() as string | null),
      },
    ],
    [],
  );

  const tableData = useMemo(() => data?.items ?? [], [data]);

  const table = useReactTable({
    data: tableData,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
    manualPagination: true,
    rowCount: data?.total ?? 0,
  });

  const { rows } = table.getRowModel();

  const tableContainerRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 48,
    overscan: 10,
  });

  const totalPages = data?.total_pages ?? 0;

  return (
    <div className="p-4">
      <h1 className="text-3xl font-bold mb-6">Authed Tokens</h1>

      {/* Token ID Lookup */}
      <div className="mb-4 flex flex-col sm:flex-row gap-3 items-end">
        <div className="w-full sm:flex-1">
          <Label htmlFor="search-token-id" className="mb-1 block text-sm">
            Search by Token ID
          </Label>
          <TextInput
            id="search-token-id"
            placeholder="Enter Authed Token UUID..."
            value={tokenIdSearch}
            onChange={(e) => setTokenIdSearch(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleTokenIdSearch()}
          />
        </div>
        <Button onClick={handleTokenIdSearch}>Lookup</Button>
      </div>
      {tokenSearchError && (
        <Alert color="failure" className="mb-4">
          {tokenSearchError}
        </Alert>
      )}

      {/* Filter Bar */}
      <div className="mb-4 flex flex-wrap gap-3 items-end">
        <div>
          <Label htmlFor="filter-origin-ip" className="mb-1 block text-sm">
            Origin IP
          </Label>
          <TextInput
            id="filter-origin-ip"
            placeholder="1.2.3.4"
            sizing="sm"
            value={originIpFilter}
            onChange={(e) => setOriginIpFilter(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
        </div>
        <div>
          <Label htmlFor="filter-writing-ua" className="mb-1 block text-sm">
            Writing UA
          </Label>
          <TextInput
            id="filter-writing-ua"
            placeholder="substring..."
            sizing="sm"
            value={writingUaFilter}
            onChange={(e) => setWritingUaFilter(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
        </div>
        <div>
          <Label htmlFor="filter-authed-ua" className="mb-1 block text-sm">
            Authed UA
          </Label>
          <TextInput
            id="filter-authed-ua"
            placeholder="substring..."
            sizing="sm"
            value={authedUaFilter}
            onChange={(e) => setAuthedUaFilter(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
        </div>
        <div>
          <Label htmlFor="filter-asn" className="mb-1 block text-sm">
            ASN
          </Label>
          <TextInput
            id="filter-asn"
            placeholder="e.g. 13335"
            sizing="sm"
            type="number"
            value={asnNumFilter}
            onChange={(e) => setAsnNumFilter(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
        </div>
        <div>
          <Label htmlFor="filter-validity" className="mb-1 block text-sm">
            Validity
          </Label>
          <Select
            id="filter-validity"
            sizing="sm"
            value={validityFilter}
            onChange={(e) => setValidityFilter(e.target.value)}
          >
            <option value="">All</option>
            <option value="true">Valid</option>
            <option value="false">Revoked</option>
          </Select>
        </div>
        <Button size="sm" onClick={handleSearch}>
          Search
        </Button>
      </div>

      {isError && (
        <Alert color="failure" className="mb-4">
          Failed to load authed tokens.
        </Alert>
      )}

      {/* Table */}
      <div
        ref={tableContainerRef}
        className="overflow-auto border border-gray-200 dark:border-gray-700 rounded-lg"
        style={{ maxHeight: "70vh" }}
      >
        <Table hoverable>
          <TableHead className="sticky top-0 z-10">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <TableHeadCell
                    key={header.id}
                    className={
                      header.column.getCanSort()
                        ? "cursor-pointer select-none"
                        : ""
                    }
                    onClick={header.column.getToggleSortingHandler()}
                    style={{ width: header.getSize() }}
                  >
                    <div className="flex items-center gap-1">
                      {flexRender(
                        header.column.columnDef.header,
                        header.getContext(),
                      )}
                      {{
                        asc: " \u2191",
                        desc: " \u2193",
                      }[header.column.getIsSorted() as string] ?? null}
                    </div>
                  </TableHeadCell>
                ))}
              </tr>
            ))}
          </TableHead>
          <TableBody className="divide-y">
            {virtualizer.getVirtualItems().length === 0 && !isLoading && (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="text-center py-8 text-gray-500"
                >
                  No tokens found.
                </TableCell>
              </TableRow>
            )}
            {isLoading && (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="text-center py-8 text-gray-500"
                >
                  Loading...
                </TableCell>
              </TableRow>
            )}
            {/* top spacer */}
            {virtualizer.getVirtualItems().length > 0 && (
              <tr>
                <td
                  style={{
                    height: virtualizer.getVirtualItems()[0]?.start ?? 0,
                  }}
                  colSpan={columns.length}
                />
              </tr>
            )}
            {virtualizer.getVirtualItems().map((virtualRow) => {
              const row = rows[virtualRow.index];
              return (
                <TableRow
                  key={row.id}
                  className="cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700"
                  onClick={() => setSelectedToken(row.original)}
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell key={cell.id}>
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext(),
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              );
            })}
            {/* bottom spacer */}
            {virtualizer.getVirtualItems().length > 0 && (
              <tr>
                <td
                  style={{
                    height:
                      virtualizer.getTotalSize() -
                      (virtualizer.getVirtualItems().at(-1)?.end ?? 0),
                  }}
                  colSpan={columns.length}
                />
              </tr>
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between mt-4">
        <span className="text-sm text-gray-700 dark:text-gray-400">
          {data
            ? `Showing ${(page - 1) * perPage + 1}-${Math.min(page * perPage, data.total)} of ${data.total}`
            : ""}
        </span>
        <div className="flex items-center gap-2">
          <Button
            size="xs"
            color="gray"
            disabled={page <= 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
          >
            Prev
          </Button>
          <span className="text-sm text-gray-700 dark:text-gray-400">
            Page {page} / {totalPages || 1}
          </span>
          <Button
            size="xs"
            color="gray"
            disabled={page >= totalPages}
            onClick={() => setPage((p) => p + 1)}
          >
            Next
          </Button>
        </div>
      </div>

      {/* Detail Modal */}
      <Modal
        show={selectedToken !== null}
        onClose={() => {
          setSelectedToken(null);
          setShowAdditionalInfo(false);
        }}
        size="4xl"
      >
        <ModalHeader className="border-gray-200">
          <div className="flex items-center gap-3">
            <span>Token Details</span>
            {selectedToken && (
              <Badge
                color={selectedToken.validity ? "success" : "failure"}
                size="sm"
              >
                {selectedToken.validity ? "Valid" : "Revoked"}
              </Badge>
            )}
          </div>
        </ModalHeader>
        <ModalBody>
          {selectedToken && (
            <div className="space-y-6">
              <p className="text-sm text-gray-500 dark:text-gray-400 font-mono">
                {selectedToken.id}
              </p>

              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                {/* Token Information */}
                <div>
                  <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-white">
                    Token Information
                  </h3>
                  <dl className="space-y-2">
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Token
                      </dt>
                      <dd className="font-mono text-sm break-all">
                        {selectedToken.token}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Created At
                      </dt>
                      <dd>{formatDateTime(selectedToken.created_at)}</dd>
                    </div>
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Authed At
                      </dt>
                      <dd>{formatDateTime(selectedToken.authed_at)}</dd>
                    </div>
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Last Wrote At
                      </dt>
                      <dd>{formatDateTime(selectedToken.last_wrote_at)}</dd>
                    </div>
                  </dl>
                </div>

                {/* Network Information */}
                <div>
                  <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-white">
                    Network Information
                  </h3>
                  <dl className="space-y-2">
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Origin IP
                      </dt>
                      <dd className="font-mono">{selectedToken.origin_ip}</dd>
                    </div>
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Reduced Origin IP
                      </dt>
                      <dd className="font-mono">
                        {selectedToken.reduced_origin_ip}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        ASN Number
                      </dt>
                      <dd className="font-mono">
                        {selectedToken.asn_num ?? "N/A"}
                      </dd>
                    </div>
                  </dl>
                </div>

                {/* User Agent Information */}
                <div>
                  <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-white">
                    User Agent Information
                  </h3>
                  <dl className="space-y-2">
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Writing UA
                      </dt>
                      <dd className="text-sm break-all">
                        {selectedToken.writing_ua}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-sm text-gray-500 dark:text-gray-400">
                        Authed UA
                      </dt>
                      <dd className="text-sm break-all">
                        {selectedToken.authed_ua ?? "N/A"}
                      </dd>
                    </div>
                  </dl>
                </div>

                {/* Additional Info */}
                <div>
                  <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-white">
                    Additional Information
                  </h3>
                  {selectedToken.additional_info ? (
                    <div>
                      <button
                        type="button"
                        onClick={() =>
                          setShowAdditionalInfo(!showAdditionalInfo)
                        }
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
                          {JSON.stringify(
                            selectedToken.additional_info,
                            null,
                            2,
                          )}
                        </pre>
                      )}
                    </div>
                  ) : (
                    <p className="text-gray-500 dark:text-gray-400">
                      No additional information available
                    </p>
                  )}
                </div>
              </div>

              {/* Actions */}
              <div>
                <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-white">
                  Actions
                </h3>
                <div className="flex flex-wrap gap-3">
                  <Button
                    color="failure"
                    size="sm"
                    onClick={handleRevokeToken}
                    disabled={selectedToken.validity === false}
                  >
                    Revoke This Token
                  </Button>
                  <Button
                    color="failure"
                    size="sm"
                    onClick={handleRevokeAllFromOriginIp}
                    disabled={selectedToken.validity === false}
                  >
                    Revoke All Tokens from Origin IP
                  </Button>
                </div>
                {selectedToken.validity === false && (
                  <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">
                    This token has already been revoked.
                  </p>
                )}
              </div>
            </div>
          )}
        </ModalBody>
      </Modal>
    </div>
  );
};

export default Page;
