import { useState } from "react";
import TasksDrawerButton from "./Button";
import TasksDrawerList from "./List";
import Menu from "@mui/material/Menu";

const TasksDrawer = () => {
  const [open, setOpen] = useState(false);

  const handleOpen = () => setOpen(true);
  const handleClose = () => setOpen(false);

  return (
    <>
      <TasksDrawerButton onClick={handleOpen} />

      <Menu
        open={open}
        onClose={handleClose}
        anchorEl={document.body}
        anchorOrigin={{ vertical: "top", horizontal: "right" }}
      >
        <TasksDrawerList />
      </Menu>
    </>
  );
};

export default TasksDrawer;
