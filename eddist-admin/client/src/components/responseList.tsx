import React from "react";
import {
  Box,
  Checkbox,
  Typography,
  IconButton,
  Menu,
  MenuItem,
  ListItemText,
  Skeleton,
} from "@mui/material";
import MoreVertIcon from "@mui/icons-material/MoreVert";

export interface Res {
  id: string;
  authorName: string;
  mail: string;
  body: string;
  createdAt: string;
  authorId: string;
  ipAddr: string;
  authedTokenId: string;
  isAbone: boolean;
  resOrder: number;
}

interface Props {
  responses: Res[];
  selectedResponses: Res[];
  setSelectedResponses: React.Dispatch<React.SetStateAction<Res[]>>;
  onClickAbon: (responseId: string) => void;
  onClickDeleteAuthedToken: (authedTokenId: string) => void;
  onClickDeleteAuthedTokensAssociatedWithIp: (authedTokenId: string) => void;
  onClickEditResponse: (response: Res) => void;
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
  const [anchorEl, setAnchorEl] = React.useState<
    (EventTarget & HTMLButtonElement) | undefined
  >();
  const [currentResponse, setCurrentResponse] = React.useState<
    Res | undefined
  >();
  const open = Boolean(anchorEl);

  const handleMenuClick = (
    event: React.MouseEvent<HTMLButtonElement, MouseEvent>,
    response: Res
  ) => {
    setAnchorEl(event.currentTarget);
    setCurrentResponse(response);
  };

  const handleMenuClose = () => {
    setAnchorEl(undefined);
    setCurrentResponse(undefined);
  };

  if (responses == null) {
    // Returns MUI like skeleton and style is below. It must be used <Skeleton /> from MUI.
    return (
      <Box bgcolor="grey.200" p={2} borderRadius={2} mb={2}>
        <Box
          display="flex"
          alignItems="center"
          mb={1}
          borderBottom={1}
          borderColor="divider"
        >
          <Checkbox />
          <Typography variant="body1" fontWeight="bold" mr={2}>
            <Skeleton />
          </Typography>
          <Typography variant="body1" mr={2}>
            <Skeleton />
          </Typography>
          <Typography variant="body2" color="textSecondary" mr={2}>
            <Skeleton />
          </Typography>
          <Typography variant="body2" color="textSecondary" mr={2}>
            <Skeleton />
          </Typography>
          <Typography variant="body2" color="textSecondary" flexGrow={1}>
            <Skeleton />
          </Typography>
          <IconButton>
            <MoreVertIcon />
          </IconButton>
        </Box>
        <Skeleton />
      </Box>
    );
  }

  return responses.map((response, idx) => (
    <Box key={response.id} bgcolor="grey.200" p={2} borderRadius={2} mb={2}>
      <Box
        display="flex"
        alignItems="center"
        mb={1}
        borderBottom={1}
        borderColor="divider"
      >
        <Checkbox
          checked={selectedResponses.includes(response)}
          onChange={() => {
            if (selectedResponses.includes(response)) {
              setSelectedResponses(
                selectedResponses.filter((res) => res !== response)
              );
            } else {
              setSelectedResponses([...selectedResponses, response]);
            }
          }}
        />
        <Typography variant="body1" fontWeight="bold" mr={2}>
          {idx + 1}
        </Typography>
        <Typography variant="body1" mr={2}>
          {response.authorName}
        </Typography>
        <Typography variant="body2" color="textSecondary" mr={2}>
          {response.mail}
        </Typography>
        <Typography variant="body2" color="textSecondary" mr={2}>
          {response.createdAt}
        </Typography>
        <Typography variant="body2" color="textSecondary" flexGrow={1}>
          ID:{response.authorId}
        </Typography>
        <IconButton onClick={(event) => handleMenuClick(event, response)}>
          <MoreVertIcon />
        </IconButton>
      </Box>
      <Box
        dangerouslySetInnerHTML={{ __html: response.body }}
        whiteSpace="pre-wrap"
      />
      <Box color="textSecondary" fontSize="small" mt={1}>
        <Typography>IP: {response.ipAddr}</Typography>
        <Typography>Authed Token: {response.authedTokenId}</Typography>
        <Typography>User Agent: Not implemented yet</Typography>
      </Box>
      <Menu anchorEl={anchorEl} open={open} onClose={handleMenuClose}>
        <MenuItem
          onClick={() => {
            if (currentResponse) {
              onClickAbon(currentResponse.id);
              handleMenuClose();
            }
          }}
        >
          <ListItemText primary="Delete Response (Abon)" />
        </MenuItem>
        <MenuItem
          disabled={currentResponse?.authedTokenId == null}
          onClick={() => {
            if (currentResponse) {
              onClickDeleteAuthedToken(currentResponse.authedTokenId);
              handleMenuClose();
            }
          }}
        >
          <ListItemText primary="Delete authed token" />
        </MenuItem>
        <MenuItem
          disabled={currentResponse?.authedTokenId == null}
          onClick={() => {
            if (currentResponse) {
              onClickDeleteAuthedTokensAssociatedWithIp(
                currentResponse.authedTokenId
              );
              handleMenuClose();
            }
          }}
        >
          <ListItemText primary="Delete authed token associated with writing origin IP of authed token" />
        </MenuItem>
        <MenuItem
          onClick={() => {
            if (currentResponse) {
              onClickEditResponse(currentResponse);
              handleMenuClose();
            }
          }}
        >
          <ListItemText primary="Edit response" />
        </MenuItem>
      </Menu>
    </Box>
  ));
};

export default ResponseList;
