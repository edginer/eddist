import { Breadcrumbs, Link, Typography } from "@mui/material";
import { useList, useUpdate } from "@refinedev/core";
import { List } from "@refinedev/mui";
import gql from "graphql-tag";
import { useNavigate, useParams } from "react-router-dom";
import ResponseList, { Res } from "../components/responseList";
import { useCallback, useState } from "react";

const GET_THREAD_QUERY = gql`
  query GetThread($boardKey: String, $threadNumber: Int) {
    boards(boardKeys: [$boardKey]) {
      threads(threadNumber: [$threadNumber]) {
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
        responses {
          id
          authorName
          mail
          body
          createdAt
          authorId
          ipAddr
          authedTokenId
          isAbone
          resOrder
        }
      }
    }
  }
`;

const DELETE_AUTHED_TOKEN = gql`
  mutation DeleteAuthedToken($data: DeleteAuthedTokenInput!) {
    deleteAuthedToken(input: $data)
  }
`;

const UPDATE_RESPONSE = gql`
  mutation UpdateResponse($data: ResInput!) {
    updateResponse(res: $data) {
      id
      authorName
      mail
      body
      isAbone
    }
  }
`;

export const Thread = () => {
  const params = useParams();

  const { data: tData } = useList({
    resource: "Threads",
    meta: {
      gqlQuery: GET_THREAD_QUERY,
      variables: {
        boardKey: params.boardKey,
        threadNumber: Number(params.threadKey),
      },
      operation: "boards",
    },
  });

  const { mutate } = useUpdate();

  const thread = tData?.data[0].threads[0];
  const navigator = useNavigate();

  const [selectedResponses, setSelectedResponses] = useState<Res[]>([]);

  const revokeAuthedToken = useCallback(
    (authedTokenId: string, usingOriginIp: boolean) => {
      mutate({
        resource: "Threads",
        id: authedTokenId,
        values: {
          tokenId: authedTokenId,
          usingOriginIp,
        },
        meta: {
          gqlMutation: DELETE_AUTHED_TOKEN,
        },
      });
    },
    [mutate]
  );

  return (
    <div>
      <List
        title={<Typography variant="h4">Thread: {thread?.title}</Typography>}
        breadcrumb={
          <Breadcrumbs sx={{ px: 3, pt: 2, cursor: "default" }}>
            <Link
              underline="hover"
              onClick={() => navigator("/dashboard/boards")}
            >
              Boards
            </Link>
            <Link
              underline="hover"
              onClick={() => navigator(`/dashboard/boards/${params.boardKey}`)}
            >
              {params.boardKey}
            </Link>
            <Typography>
              {thread?.title} ({thread?.threadNumber})
            </Typography>
          </Breadcrumbs>
        }
      >
        <ResponseList
          responses={thread?.responses as Res[]}
          selectedResponses={selectedResponses}
          setSelectedResponses={setSelectedResponses}
          onClickAbon={(responseId: string) => {
            mutate({
              resource: "Threads",
              id: responseId,
              values: {
                id: responseId,
                isAbone: true,
              },
              meta: {
                gqlMutation: UPDATE_RESPONSE,
              },
            });
          }}
          onClickDeleteAuthedToken={(authedTokenId: string) => {
            revokeAuthedToken(authedTokenId, false);
          }}
          onClickDeleteAuthedTokensAssociatedWithIp={(
            authedTokenId: string
          ) => {
            revokeAuthedToken(authedTokenId, true);
          }}
          onClickEditResponse={(response) => {
            throw new Error("Function not implemented.");
          }}
        />
      </List>
    </div>
  );
};
