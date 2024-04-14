/**
 * Button component for Youtube login functionality to embed in the
 * navbar of the react-admin application.
 */

import React from "react";
import { Button, useDataProvider } from "react-admin";

import LoginIcon from "@mui/icons-material/Login";

export const YoutubeLoginButton: React.FC = () => {
  const dataProvider = useDataProvider();

  const goToYoutubeLogin = React.useCallback(() => {
    dataProvider.youtubeLogin().then((url: string) => {
      window.location.href = url;
    });
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
