/**
 * Button component for Twitch login functionality to embed in the
 * navbar of the react-admin application.
 */

import React from "react";
import { Button, useDataProvider } from "react-admin";

import LoginIcon from "@mui/icons-material/Login";

export const TwitchLoginButton: React.FC = () => {
  const dataProvider = useDataProvider();

  const goToTwitchLogin = React.useCallback(() => {
    dataProvider.twitchLogin().then((url: string) => {
      window.location.href = url;
    });
  }, [dataProvider]);

  return (
    <Button
      color="primary"
      variant="contained"
      label="Log in with Twitch"
      onClick={goToTwitchLogin}
      startIcon={<LoginIcon />}
    />
  );
};
