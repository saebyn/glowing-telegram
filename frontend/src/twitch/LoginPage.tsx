/**
 * A page within the larger react-admin application that is shown when the user
 * gets redirected back to the frontend after logging in with Twitch.
 */

import { Card, CardContent, Typography } from "@mui/material";
import React from "react";
import { Title, useDataProvider, useNotify, useRedirect } from "react-admin";
import { useLocation } from "react-router-dom";

export const TwitchLoginPage: React.FC = () => {
  const location = useLocation();
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const redirect = useRedirect();

  React.useEffect(() => {
    const search = new URLSearchParams(location.search);
    const code = search.get("code");

    if (!code) {
      notify("Failed to log in with Twitch", {
        type: "warning",
        messageArgs: { smart_count: 1 },
      });
      return;
    }

    dataProvider
      .twitchCallback(code)
      .then(() => {
        notify("Logged in with Twitch", {
          type: "info",
          messageArgs: { smart_count: 1 },
        });

        redirect("/");
      })
      .catch(() => {
        notify("Failed to log in with Twitch", {
          type: "warning",
          messageArgs: { smart_count: 1 },
        });
      });
  }, [location, dataProvider, notify, redirect]);

  return (
    <Card>
      <Title title="Twitch Login" />
      <CardContent>
        <Typography variant="body1">Logging in with Twitch...</Typography>
      </CardContent>
    </Card>
  );
};
