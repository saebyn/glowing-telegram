/**
 * Button component for Youtube login functionality to embed in the
 * navbar of the react-admin application.
 */

import React from "react";
import { Button, useDataProvider } from "react-admin";

import LoginIcon from "@mui/icons-material/Login";
import { AuthorizationData } from "../dataProvider";

export const YoutubeLoginButton: React.FC = () => {
  const dataProvider = useDataProvider();

  const goToYoutubeLogin = React.useCallback(() => {
    dataProvider
      .youtubeLogin()
      .then(
        ({
          authorize_url,
          csrf_state,
          pkce_code_verifier,
        }: AuthorizationData) => {
          // TODO: Don't pass this information through the frontend
          localStorage.setItem("csrf_state", csrf_state);
          localStorage.setItem("pkce_code_verifier", pkce_code_verifier);

          window.open(
            authorize_url,
            "_blank",
            "noopener,noreferrer,popup,toolbar=0,width=600,height=600",
          );
        },
      );
  }, [dataProvider]);

  return (
    <Button
      color="primary"
      variant="contained"
      label="Log in with Youtube"
      onClick={goToYoutubeLogin}
      startIcon={<LoginIcon />}
    />
  );
};
