import { useEffect } from "react";
import { useDataProvider } from "react-admin";
import { TaskStatusWebsocketMessage } from "../websocket";

function useTasksStatusSubscription(
  dispatch: (action: TaskStatusWebsocketMessage) => void
) {
  const dataProvider = useDataProvider();

  useEffect(() => {
    console.log("subscribing to task status");
    const subscriptionPromise = dataProvider.subscribeToTaskStatus(dispatch);

    return () => {
      console.log("unsubscribing from task status");
      subscriptionPromise.then((unsubscribe: () => void) => {
        unsubscribe();
      });
    };
  }, [dataProvider, dispatch]);
}

export default useTasksStatusSubscription;
