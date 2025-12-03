import { Dropdown, DropdownItem } from "flowbite-react";
import React from "react";
import { components } from "~/openapi/schema";

interface Props {
  responses?: components["schemas"]["ArchivedRes"][];
  adminResponses: components["schemas"]["ArchivedAdminRes"][];
  selectedResponsesOrder: number[];
  setSelectedResponsesOrder: React.Dispatch<React.SetStateAction<number[]>>;
  onClieckAbon: (resOrder: number) => void;
  onClickDeleteAuthedToken: (authedToken: string) => void;
  onClickDeleteAuthedTokensAssociatedWithIp: (authedToken: string) => void;
  onClickEditResponse: (resOrder: number) => void;
}

const DatArchiveResponseList = ({
  responses,
  adminResponses,
  selectedResponsesOrder,
  setSelectedResponsesOrder,
  onClieckAbon,
  onClickDeleteAuthedToken,
  onClickDeleteAuthedTokensAssociatedWithIp,
  onClickEditResponse,
}: Props) => {
  return adminResponses.map((response, idx) => {
    return (
      <div
        key={`${response.date}:${response.authed_token_id}`}
        className="bg-gray-200 p-4 rounded-lg mb-4"
      >
        <div className="flex items-center mb-2 border-b border-gray-200">
          <input
            type="checkbox"
            className="mr-2"
            onClick={() => {
              if (
                selectedResponsesOrder.find((order) => order === idx) != null
              ) {
                setSelectedResponsesOrder((s) =>
                  s.filter((order) => order !== idx)
                );
              } else {
                setSelectedResponsesOrder((s) => [...s, idx]);
              }
            }}
          />

          <span className="font-bold mr-2">{idx + 1}</span>
          <span className="mr-2">{response.name}</span>
          <span className="text-gray-500 mr-2">{response.mail}</span>
          <span className="text-gray-500 mr-2">{response.date}</span>
          <span className="text-gray-500 grow">ID:{response.author_id}</span>
          {responses && responses[idx] && responses[idx].is_abone && (
            <div>
              <hr className="mr-2" />
              <span className="text-gray-500 mr-2">
                This responses is deleted.
              </span>
            </div>
          )}
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
              <DropdownItem onClick={() => onClickEditResponse(idx)}>
                Edit
              </DropdownItem>
              <DropdownItem onClick={() => onClieckAbon(idx)}>
                Abon
              </DropdownItem>
              <DropdownItem
                onClick={() =>
                  onClickDeleteAuthedToken(response.authed_token_id)
                }
              >
                Delete Authed Token
              </DropdownItem>
              <DropdownItem
                onClick={() =>
                  onClickDeleteAuthedTokensAssociatedWithIp(
                    response.authed_token_id
                  )
                }
              >
                Delete Authed Tokens Associated With IP
              </DropdownItem>
            </Dropdown>
          </div>
        </div>
        <div className="p-2">{response.body}</div>
        {responses &&
          responses[idx] &&
          !responses[idx].is_abone &&
          (responses[idx].body !== response.body ||
            responses[idx].name !== response.name ||
            responses[idx].mail !== response.mail) && (
            <div>
              <hr className="bg-gray-800 border border-black" />
              <div className="text-gray-500 mr-2">(Edited response)</div>
              <span className="text-gray-500 mr-2">{responses[idx].name}</span>
              <span className="text-gray-500 mr-2">{responses[idx].mail}</span>
              <span className="text-gray-500 mr-2">{responses[idx].date}</span>
              <span className="text-gray-500 grow">
                ID:{responses[idx].author_id}
              </span>
              <div className="p-2">{responses[idx].body}</div>
            </div>
          )}
        <div className="text-gray-500 text-sm mt-2">
          <p>IP: {response.ip_addr}</p>
          <p>Authed Token ID: {response.authed_token_id}</p>
        </div>
      </div>
    );
  });
};

export default DatArchiveResponseList;
