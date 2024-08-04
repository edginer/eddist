import { Breadcrumbs, Link, Typography } from "@mui/material";
import { DataGrid } from "@mui/x-data-grid";
import { useList } from "@refinedev/core";
import { List, useDataGrid } from "@refinedev/mui";
import gql from "graphql-tag";
import { useMemo } from "react";
import { useNavigate, useParams } from "react-router-dom";

const GET_THREADLIST_QUERY = gql`
  query GetThreadList($boardKey: String) {
    boards(boardKeys: [$boardKey]) {
      threads(threadNumber: []) {
        id
        boardId
        threadNumber
        lastModified
        sageLastModified
        title
        authedTokenId
        metadent
        responseCount
        noPool
        archived
        active
      }
    }
  }
`;

export const ThreadList = () => {
  const params = useParams();

  const { data: tData } = useList({
    resource: "Threads",
    meta: {
      gqlQuery: GET_THREADLIST_QUERY,
      variables: { boardKey: params.boardKey },
      operation: "boards",
      fields: [
        {
          operation: "threads",
          fields: ["id", "threadNumber", "title", "responseCount"],
        },
      ],
    },
  });
  const threads = tData?.data[0].threads ?? [];

  const navigator = useNavigate();

  const columns = useMemo(
    () => [
      {
        field: "id",
        headerName: "Id",
        minWidth: 320,
      },
      {
        field: "threadNumber",
        headerName: "Thread Number",
        minWidth: 140,
      },
      {
        field: "title",
        headerName: "Title",
        minWidth: 350,
      },
      {
        field: "responseCount",
        headerName: "Res Count",
        minWidth: 50,
      },
    ],
    []
  );

  return (
    <div>
      <List
        title={<Typography variant="h4">Threads</Typography>}
        breadcrumb={
          <Breadcrumbs sx={{ px: 3, pt: 2, cursor: "default" }}>
            <Link underline="hover" onClick={() => navigator("/boards")}>
              Boards
            </Link>
            <Typography>{params.boardKey}</Typography>
          </Breadcrumbs>
        }
      >
        <DataGrid
          rows={threads}
          columns={columns}
          autoHeight
          onRowClick={(x) => {
            navigator(`/boards/${params.boardKey}/${x.row.threadNumber}`);
          }}
          checkboxSelection
          sx={{
            "& .MuiDataGrid-cell:focus, & .MuiDataGrid-cell:focus-within": {
              outline: "none",
            },
          }}
          disableRowSelectionOnClick
        />
      </List>
    </div>
  );
};
