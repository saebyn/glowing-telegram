import Box from "@mui/material/Box";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem/ListItem";
import ListItemText from "@mui/material/ListItemText";
import ListItemAvatar from "@mui/material/ListItemAvatar";
import Avatar from "@mui/material/Avatar";
import Typography from "@mui/material/Typography";
import Switch from "@mui/material/Switch";
import ProcessingIcon from "@mui/icons-material/Loop";
import DoneIcon from "@mui/icons-material/Done";
import ErrorIcon from "@mui/icons-material/Error";
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty";
import Button from "@mui/material/Button";
import { LoadingIndicator } from "react-admin";
import useTasks from "./useTasks";
import { FC, useRef } from "react";
import Notifications from "./Notifications";
import { TaskStatus, TaskSummary } from "../types";
import IconButton from "@mui/material/IconButton";
import VisibilityIcon from "@mui/icons-material/Visibility";
import useTheme from "@mui/material/styles/useTheme";
import Badge from "@mui/material/Badge";
import { green, orange, red, blue } from "@mui/material/colors";

const containerStyles = {
  minWidth: 250,
  padding: "2em",
};

const statusIcons: Record<TaskStatus, JSX.Element> = {
  processing: <ProcessingIcon titleAccess="Processing" />,
  complete: <DoneIcon titleAccess="Complete" />,
  failed: <ErrorIcon titleAccess="Failed" />,
  queued: <HourglassEmptyIcon titleAccess="Queued" />,
  invalid: <ErrorIcon titleAccess="Invalid" />,
};

const statusColors: Record<TaskStatus, string> = {
  processing: orange[500],
  complete: green[500],
  failed: red[500],
  queued: blue[500],
  invalid: red[500],
};

interface TaskProps {
  task: TaskSummary;
  lastViewedTaskTimestamp: Date;
  markViewed: (id: number) => void;
}

const Task: FC<TaskProps> = ({
  task,

  lastViewedTaskTimestamp,
  markViewed,
}) => {
  const theme = useTheme();

  const timestamp = task.last_updated
    ? new Date(task.last_updated).toLocaleString()
    : "unknown";

  const newSinceLastView =
    new Date(task.last_updated) > lastViewedTaskTimestamp;

  return (
    <ListItem
      key={task.id}
      sx={{
        backgroundColor: newSinceLastView ? theme.palette.action.selected : "",
      }}
      secondaryAction={
        <IconButton
          edge="end"
          aria-label="mark viewed"
          onClick={() => markViewed(task.id)}
        >
          <VisibilityIcon />
        </IconButton>
      }
    >
      <ListItemAvatar>
        <Badge
          color="secondary"
          variant="dot"
          invisible={!task.has_next_task}
          overlap="circular"
          anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        >
          <Avatar
            alt={task.status}
            variant="rounded"
            sx={{ bgcolor: statusColors[task.status] }}
          >
            {statusIcons[task.status]}
          </Avatar>
        </Badge>
      </ListItemAvatar>
      <ListItemText
        primary={task.title || task.id}
        secondary={
          <>
            <Typography variant="body2" color="text.primary">
              {timestamp}
            </Typography>

            {task.has_next_task && (
              <Typography variant="caption">
                More tasks will start when this one finishes
              </Typography>
            )}
          </>
        }
      />
    </ListItem>
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
