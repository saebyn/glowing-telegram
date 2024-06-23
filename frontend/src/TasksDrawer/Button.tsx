/**
 * Button component for the TasksDrawer.
 *
 * Show the count of tasks whose status has changed since the last time the user viewed the TasksDrawer.
 */
import { Button, Badge } from "@mui/material";
import ListIcon from "@mui/icons-material/List";
import useTaskStatusSubscription from "./useTasksStatusSubscription";
import { Reducer, useCallback, useReducer } from "react";
import { TaskStatusWebsocketMessage } from "../websocket";

interface Props {
  onClick: () => void;
}

interface TasksStatus {
  count: number;
}

type Action = TaskStatusWebsocketMessage | { event: "reset" };

const TasksDrawerButton = ({ onClick }: Props) => {
  const [tasksStatus, dispatch] = useReducer<Reducer<TasksStatus, Action>>(
    (state, action) => {
      switch (action.event) {
        case "task_status_change":
          return {
            count: state.count + 1,
          };

        case "reset":
          return {
            count: 0,
          };

        default:
          return state;
      }
    },
    {
      count: 0,
    }
  );

  useTaskStatusSubscription(dispatch);

  const handleClick = useCallback(() => {
    dispatch({ event: "reset" });
    onClick();
  }, [dispatch, onClick]);

  return (
    <Button color="primary" onClick={handleClick}>
      <Badge badgeContent={tasksStatus.count} color="primary">
        <ListIcon />
      </Badge>
    </Button>
  );
};

export default TasksDrawerButton;
