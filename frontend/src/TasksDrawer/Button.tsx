/**
 * Button component for the TasksDrawer.
 *
 * Show the count of tasks whose status has changed since the last time the user viewed the TasksDrawer.
 */
import { Button, Badge } from "@mui/material";
import ListIcon from "@mui/icons-material/List";
import useTasks from "./useTasks";

interface Props {
  onClick: () => void;
}

const TasksDrawerButton = ({ onClick }: Props) => {
  const { count } = useTasks();

  return (
    <Button color="primary" onClick={onClick}>
      <Badge badgeContent={count} color="primary">
        <ListIcon />
      </Badge>
    </Button>
  );
};

export default TasksDrawerButton;
