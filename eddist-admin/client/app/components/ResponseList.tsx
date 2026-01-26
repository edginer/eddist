import { Dropdown, DropdownItem } from "flowbite-react";
import React, { useState } from "react";
import { Link } from "react-router";
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
  const [expandedClientInfo, setExpandedClientInfo] = useState<Set<string>>(
    new Set(),
  );

  const toggleClientInfo = (responseId: string) => {
    setExpandedClientInfo((prev) => {
      const next = new Set(prev);
      if (next.has(responseId)) {
        next.delete(responseId);
      } else {
        next.add(responseId);
      }
      return next;
    });
  };

  return responses.map((response, idx) => (
    <div key={response.id} className="bg-gray-200 p-4 rounded-lg mb-4">
      <div className="flex items-center mb-2 border-b border-gray-200">
        {selectedResponses && setSelectedResponses && (
          <input
            type="checkbox"
            className="mr-2"
            id={`${response.id}`}
            onClick={() => {
              if (selectedResponses.find((r) => r.id === response.id) != null) {
                setSelectedResponses((s) =>
                  s.filter((res) => res.id !== response.id),
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
                  response.authed_token_id!!,
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
        <p>
          Authed Token ID:{" "}
          {response.authed_token_id ? (
            <Link
              to={`/dashboard/authed-token?token=${response.authed_token_id}`}
              className="text-blue-600 hover:underline"
            >
              {response.authed_token_id}
            </Link>
          ) : (
            "N/A"
          )}
        </p>
        <div className="mt-2">
          <button
            onClick={() => toggleClientInfo(response.id)}
            className="flex items-center text-gray-600 hover:text-gray-800"
          >
            <svg
              className={`w-4 h-4 mr-1 transition-transform ${
                expandedClientInfo.has(response.id) ? "rotate-90" : ""
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
            Client Info
          </button>
          {expandedClientInfo.has(response.id) && (
            <div className="ml-5 mt-2 p-2 bg-gray-100 rounded">
              <p>User Agent: {response.client_info.user_agent}</p>
              <p>ASN: {response.client_info.asn_num}</p>
              <p>IP Address: {response.client_info.ip_addr}</p>
              {response.client_info.tinker && (
                <div className="mt-2 pt-2 border-t border-gray-300">
                  <p className="font-semibold">Tinker:</p>
                  <p className="ml-2">
                    Authed Token: {response.client_info.tinker.authed_token}
                  </p>
                  <p className="ml-2">
                    Wrote Count: {response.client_info.tinker.wrote_count}
                  </p>
                  <p className="ml-2">
                    Created Thread Count:{" "}
                    {response.client_info.tinker.created_thread_count}
                  </p>
                  <p className="ml-2">
                    Level: {response.client_info.tinker.level}
                  </p>
                  <p className="ml-2">
                    Last Level Up:{" "}
                    {new Date(
                      response.client_info.tinker.last_level_up_at * 1000,
                    ).toLocaleString()}
                  </p>
                  <p className="ml-2">
                    Last Wrote:{" "}
                    {new Date(
                      response.client_info.tinker.last_wrote_at * 1000,
                    ).toLocaleString()}
                  </p>
                  {response.client_info.tinker.last_created_thread_at && (
                    <p className="ml-2">
                      Last Created Thread:{" "}
                      {new Date(
                        response.client_info.tinker.last_created_thread_at *
                          1000,
                      ).toLocaleString()}
                    </p>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  ));
};

export default ResponseList;
