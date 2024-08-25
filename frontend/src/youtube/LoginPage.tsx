/**
 * A page within the larger react-admin application that is shown when the user
 * gets redirected back to the frontend after logging in with Youtube.
 */

import { Card, CardContent, Typography } from "@mui/material";
import React from "react";
import { Title, useDataProvider, useNotify, useRedirect } from "react-admin";
import { useLocation } from "react-router-dom";

export const YoutubeLoginPage: React.FC = () => {
  const location = useLocation();
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const redirect = useRedirect();

  React.useEffect(() => {
    const search = new URLSearchParams(location.search);
    const code = search.get("code");

    if (!code) {
      notify("Failed to log in with Youtube", {
        type: "warning",
        messageArgs: { smart_count: 1 },
      });
      return;
    }

    const state = search.get("state");

    if (!state) {
      notify("Failed to log in with Youtube", {
        type: "warning",
        messageArgs: { smart_count: 1 },
      });
      return;
    }

    // TODO: Don't pass this information through the frontend
    const csrfState = localStorage.getItem("csrf_state")!;
    const pkceCodeVerifier = localStorage.getItem("pkce_code_verifier")!;

    dataProvider
      .youtubeCallback(code, state, csrfState, pkceCodeVerifier)
      .then(() => {
        notify("Logged in with Youtube. You can close this window now.", {
          type: "info",
          messageArgs: { smart_count: 1 },
        });
      })
      .catch(() => {
        notify("Failed to log in with Youtube", {
          type: "warning",
          messageArgs: { smart_count: 1 },
        });
      });
  }, [location, dataProvider, notify, redirect]);

  return (
    <Card>
      <Title title="Youtube Login" />
      <CardContent>
        <Typography variant="body1">Logging in with Youtube...</Typography>
      </CardContent>
    </Card>
  );
};
