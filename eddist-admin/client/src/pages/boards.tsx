import { Typography } from "@mui/material";
import { List, useDataGrid } from "@refinedev/mui";
import gql from "graphql-tag";
import { DataGrid, GridColDef } from "@mui/x-data-grid";
import { useMemo } from "react";
import { useNavigate } from "react-router-dom";

const GET_BOARDS_QUERY = gql`
  query GetBoards($boardKeys: [String]) {
    boards(boardKeys: $boardKeys) {
      id
      name
      boardKey
      defaultName
      threadCount
    }
  }
`;

interface GridCol {
  field: string;
  headerName: string;
  minWidth: number;
  type?: string;
}

export const Boards = () => {
  const { dataGridProps } = useDataGrid({
    resource: "Boards",
    meta: { gqlQuery: GET_BOARDS_QUERY, variables: { boardKeys: [] } },
  });
  const navigator = useNavigate();

  const columns = useMemo<GridColDef<GridCol>[]>(
    () => [
      {
        field: "id",
        headerName: "Id",
        type: "string",
        minWidth: 320,
      },
      {
        field: "boardKey",
        headerName: "Board Key",
        type: "string",
        minWidth: 75,
      },
      {
        field: "name",
        headerName: "name",
        minWidth: 200,
      },
      {
        field: "threadCount",
        headerName: "Thread Count",
        minWidth: 50,
      },
    ],
    []
  );

  return (
    <div>
      <List title={<Typography variant="h4">Boards</Typography>}>
        <DataGrid
          {...dataGridProps}
          columns={columns}
          autoHeight
          onRowClick={(x) => {
            navigator(`/dashboard/boards/${x.row.boardKey}`);
          }}
          sx={{
            "& .MuiDataGrid-cell:focus, & .MuiDataGrid-cell:focus-within": {
              outline: "none",
            },
          }}
        />
      </List>
    </div>
  );
};
