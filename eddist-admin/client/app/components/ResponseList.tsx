import { Dropdown, DropdownItem } from "flowbite-react";
import React from "react";
import {
  Res,
  ResInput,
} from "~/routes/dashboard.boards_.$boardKey_.threads.$threadId";

interface Props {
  responses: Res[];
  selectedResponses?: ResInput[];
  setSelectedResponses?: React.Dispatch<React.SetStateAction<ResInput[]>>;
  onClickAbon?: (responseId: string) => void;
  onClickDeleteAuthedToken: (authedToken: string) => void;
  onClickDeleteAuthedTokensAssociatedWithIp: (authedToken: string) => void;
  onClickEditResponse?: (response: ResInput) => void;
}

const ResponseList = ({
  responses,
  selectedResponses,
  setSelectedResponses,
  onClickAbon,
  onClickDeleteAuthedToken,
  onClickDeleteAuthedTokensAssociatedWithIp,
  onClickEditResponse,
}: Props) => {
  return responses.map((response, idx) => (
    <div key={response.id} className="bg-gray-200 p-4 rounded-lg mb-4">
      <div className="flex items-center mb-2 border-b">
        {selectedResponses && setSelectedResponses && (
          <input
            type="checkbox"
            className="mr-2"
            id={`${response.id}`}
            onClick={() => {
              if (selectedResponses.find((r) => r.id === response.id) != null) {
                setSelectedResponses((s) =>
                  s.filter((res) => res.id !== response.id)
                );
              } else {
                setSelectedResponses((s) => [
                  ...s,
                  {
                    author_name: response.author_name ?? undefined,
                    mail: response.mail ?? undefined,
                    body: response.body,
                    id: response.id,
                  },
                ]);
              }
            }}
          />
        )}
        <span className="font-bold mr-2">{idx + 1}</span>
        <span className="mr-2">{response.author_name}</span>
        <span className="text-gray-500 mr-2">{response.mail}</span>
        <span className="text-gray-500 mr-2">{response.created_at}</span>
        <span className="text-gray-500 grow">ID:{response.author_id}</span>
        <div>
          <Dropdown
            arrowIcon={false}
            label={
              <svg
                className="w-5 h-5"
                aria-hidden="true"
                xmlns="http://www.w3.org/2000/svg"
                fill="currentColor"
                viewBox="0 0 16 3"
              >
                <path d="M2 0a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Zm6.041 0a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3ZM14 0a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Z" />
              </svg>
            }
            inline
          >
            {onClickAbon && (
              <DropdownItem
                onClick={() => {
                  onClickAbon(response.id);
                }}
              >
                Delete Response (Abon)
              </DropdownItem>
            )}
            <DropdownItem
              disabled={response.authed_token_id == null}
              onClick={() => {
                onClickDeleteAuthedToken(response.authed_token_id!!);
              }}
            >
              Delete authed token
            </DropdownItem>
            <DropdownItem
              disabled={response.authed_token_id == null}
              onClick={() => {
                onClickDeleteAuthedTokensAssociatedWithIp(
                  response.authed_token_id!!
                );
              }}
            >
              Delete authed token associated with writing origin ip of authed
              token
            </DropdownItem>
            {onClickEditResponse && (
              <DropdownItem
                onClick={() => {
                  onClickEditResponse({
                    author_name: response.author_name ?? undefined,
                    mail: response.mail ?? undefined,
                    body: response.body,
                    id: response.id,
                  });
                }}
              >
                Edit response
              </DropdownItem>
            )}
          </Dropdown>
        </div>
      </div>
      <div
        className="whitespace-pre-wrap"
        dangerouslySetInnerHTML={{ __html: response.body }}
      />
      <div className="text-gray-500 text-sm mt-2">
        <p>IP: {response.ip_addr}</p>
        <p>Authed Token ID: {response.authed_token_id}</p>
        <p>User Agent: {response.client_info.user_agent}</p>
        <p>ASN: {response.client_info.asn_num}</p>
      </div>
    </div>
  ));
};

export default ResponseList;
