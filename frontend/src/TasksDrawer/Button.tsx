/**
 * Button component for the TasksDrawer.
 *
 * Show the count of tasks whose status has changed since the last time the user viewed the TasksDrawer.
 */
import { Button, Badge } from "@mui/material";
import ListIcon from "@mui/icons-material/List";
import { useGetList, useStore } from "react-admin";

interface Props {
  onClick: () => void;
}

const TasksDrawerButton = ({ onClick }: Props) => {
  const [seenTasks] = useStore("seenTasks", [] as string[]);

  const { data: tasks } = useGetList("tasks");

  const count = tasks
    ? tasks.filter((task: any) => !seenTasks.includes(task.id)).length
    : 0;

  return (
    <Button color="primary" onClick={onClick}>
      <Badge badgeContent={count} color="primary">
        <ListIcon />
      </Badge>
    </Button>
  );
};

export default TasksDrawerButton;
