import Box from "@mui/material/Box";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem/ListItem";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemText from "@mui/material/ListItemText";
import ListItemIcon from "@mui/material/ListItemIcon";
import Typography from "@mui/material/Typography";
import Switch from "@mui/material/Switch";
import ProcessingIcon from "@mui/icons-material/Loop";
import DoneIcon from "@mui/icons-material/Done";
import ErrorIcon from "@mui/icons-material/Error";
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty";
import Button from "@mui/material/Button";
import { LoadingIndicator } from "react-admin";
import useTasks from "./useTasks";
import { useRef } from "react";
import Notifications from "./Notifications";

const containerStyles = {
  minWidth: 250,
  padding: "2em",
};

const statusIcons = {
  processing: <ProcessingIcon />,
  complete: <DoneIcon />,
  failed: <ErrorIcon />,
  queued: <HourglassEmptyIcon />,
} as const;

const Task = ({
  task,

  lastViewedTaskTimestamp,
  markViewed,
}: any) => {
  const timestamp = task.last_updated
    ? new Date(task.last_updated).toLocaleString()
    : "unknown";

  return (
    <ListItemButton
      key={task.id}
      selected={task.last_updated > lastViewedTaskTimestamp}
      onClick={() => markViewed(task.id)}
    >
      <ListItemIcon>
        {statusIcons[task.status as keyof typeof statusIcons] ||
          statusIcons.queued}
      </ListItemIcon>
      <ListItemText
        primary={task.title || task.id}
        secondary={`${task.status} (${task.id}) @ ${timestamp}`}
      />
    </ListItemButton>
  );
};

const TasksDrawerList = () => {
  const {
    lastViewedTaskTimestamp,
    tasks,
    isLoading,
    markAllViewed,
    markViewed,
    allViewed,
    refetch,
    toggleHidden,
    hiddenTasks,
  } = useTasks();

  const containerRef = useRef<HTMLDivElement>(null);

  const backToTop = () => {
    // Scroll to the top of the list
    if (containerRef.current) {
      containerRef.current.scrollIntoView({ behavior: "smooth" });
    }
  };

  if (!open) {
    return null;
  }

  return (
    <Box sx={containerStyles} ref={containerRef}>
      <Typography variant="h6" component="div">
        Tasks
      </Typography>

      {isLoading && <LoadingIndicator />}

      <Typography variant="subtitle1">
        {tasks ? `${tasks.length} tasks` : "Loading tasks..."}
      </Typography>
      <Button onClick={markAllViewed} disabled={allViewed}>
        Mark all as viewed
      </Button>
      <Button onClick={() => refetch()}>Refresh</Button>
      <Switch checked={hiddenTasks} onChange={toggleHidden} />
      <Typography variant="caption">Hide viewed</Typography>

      <List>
        {tasks.length === 0 && (
          <ListItem>
            <ListItemText primary="No tasks" />
          </ListItem>
        )}
        {tasks.map((task) => (
          <Task
            task={task}
            key={task.id}
            lastViewedTaskTimestamp={lastViewedTaskTimestamp}
            markViewed={markViewed}
          />
        ))}
      </List>

      <Typography variant="subtitle1">
        {tasks ? `${tasks.length} tasks` : "Loading tasks..."}
      </Typography>

      <Notifications />

      <Button onClick={backToTop}>Back to top</Button>
    </Box>
  );
};

export default TasksDrawerList;
