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
import { LoadingIndicator, useGetList, useStore } from "react-admin";

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

const TasksDrawerList = () => {
  const [hideViewed, setHideViewed] = useStore("hideViewedTasks", false);
  const [viewedTasks, setViewedTasks] = useStore("viewedTasks", [] as string[]);

  const { data: tasks, refetch, isLoading } = useGetList("tasks");

  const handleMarkAllViewed = () => {
    if (tasks) {
      setViewedTasks(tasks.map((task: any) => task.id));
    }
  };

  const handleMarkViewed = (taskId: string) => {
    setViewedTasks((ids) => [...ids, taskId]);
  };

  const allViewed = (tasks || []).every((task: any) =>
    viewedTasks.includes(task.id)
  );

  const handleRefresh = () => {
    refetch();
  };

  const handleToggleHideViewed = () => {
    setHideViewed(!hideViewed);
  };

  if (!open) {
    return null;
  }

  const filteredTasks = (tasks || []).filter((task: any) =>
    hideViewed ? !viewedTasks.includes(task.id) : true
  );

  return (
    <Box sx={containerStyles}>
      <Typography variant="h6" component="div">
        Tasks
      </Typography>

      {isLoading && <LoadingIndicator />}

      <List>
        {filteredTasks.length === 0 && (
          <ListItem>
            <ListItemText primary="No tasks" />
          </ListItem>
        )}
        {filteredTasks.map((task) => (
          <ListItemButton
            key={task.id}
            selected={!viewedTasks.includes(task.id)}
            onClick={() => handleMarkViewed(task.id)}
          >
            <ListItemIcon>
              {statusIcons[task.status as keyof typeof statusIcons] ||
                statusIcons.queued}
            </ListItemIcon>
            <ListItemText
              primary={task.title || task.id}
              secondary={`${task.status} (${task.id})`}
            />
          </ListItemButton>
        ))}
      </List>

      <Typography variant="subtitle1">
        {tasks ? `${tasks.length} tasks` : "Loading tasks..."}
      </Typography>

      <Button onClick={handleMarkAllViewed} disabled={allViewed}>
        Mark all as viewed
      </Button>
      <Button onClick={handleRefresh}>Refresh</Button>
      <Switch checked={hideViewed} onChange={handleToggleHideViewed} />
      <Typography variant="caption">Hide viewed</Typography>
    </Box>
  );
};

export default TasksDrawerList;
