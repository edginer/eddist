import { useTranslate, useLogin } from "@refinedev/core";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import { ThemedTitleV2 } from "@refinedev/mui";

export const Login: React.FC = () => {
  const { mutate: login } = useLogin();

  return (
    <Container
      style={{
        height: "100vh",
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
      }}
    >
      <Box
        display="flex"
        gap="36px"
        justifyContent="center"
        flexDirection="column"
      >
        <ThemedTitleV2
          collapsed={false}
          wrapperStyles={{
            fontSize: "22px",
            justifyContent: "center",
          }}
        />

        <Button
          style={{ width: "240px" }}
          size="large"
          variant="contained"
          onClick={() => login({})}
        >
          Sign in
        </Button>
        <Typography align="center" color={"text.secondary"} fontSize="12px">
          Powered by
          <img
            style={{ padding: "0 5px" }}
            alt="Keycloak"
            src="https://refine.ams3.cdn.digitaloceanspaces.com/superplate-auth-icons%2Fkeycloak.svg"
          />
          Keycloak
        </Typography>
      </Box>
    </Container>
  );
};
