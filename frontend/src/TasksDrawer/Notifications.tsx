import React from "react";
import Alert from "@mui/material/Alert";

const Notifications: React.FC = () => {
  if (!("Notification" in window)) {
    return (
      <Alert severity="error">
        This browser does not support desktop notifications
      </Alert>
    );
  }

  if (Notification.permission === "default") {
    return (
      <Alert severity="info" onClick={() => Notification.requestPermission()}>
        Click here to enable desktop notifications
      </Alert>
    );
  }

  if (Notification.permission === "denied") {
    return <Alert severity="error">Desktop notifications are disabled</Alert>;
  }

  if (Notification.permission === "granted") {
    return <Alert severity="success">Desktop notifications are enabled</Alert>;
  }
};

export default Notifications;
