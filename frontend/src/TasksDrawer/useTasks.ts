import { useGetList, useStore } from "react-admin";

interface Task {
  id: number;
  last_updated: number;
}

const useTasks = () => {
  const [hideViewed, setHideViewed] = useStore("hideViewedTasks", false);

  const [rawLastViewedTaskTimestamp, setRawLastViewedTaskTimestamp] = useStore(
    "lastViewedTaskTimestamp",
    ""
  );

  const lastViewedTaskTimestamp = new Date(rawLastViewedTaskTimestamp);

  const setLastViewedTaskTimestamp = (timestamp: Date) => {
    setRawLastViewedTaskTimestamp(timestamp.toISOString());
  };

  const { data: tasks, refetch, isLoading } = useGetList<Task>("tasks");

  const handleToggleHideViewed = () => {
    setHideViewed((hideViewed) => !hideViewed);
  };

  const handleMarkAllViewed = () => {
    if (tasks && tasks.length > 0) {
      setLastViewedTaskTimestamp(new Date(tasks[0].last_updated));
    }
  };

  const handleMarkViewed = (taskId: number) => {
    if (tasks) {
      const task = tasks.find((t) => t.id === taskId);
      if (task) {
        setLastViewedTaskTimestamp(new Date(task.last_updated));
      }
    }
  };

  const allViewed = tasks
    ? tasks.every(
        (task) => new Date(task.last_updated) <= lastViewedTaskTimestamp
      )
    : false;

  const filteredTasks = (tasks || [])
    .filter((task: Task) =>
      hideViewed ? new Date(task.last_updated) > lastViewedTaskTimestamp : true
    )
    /**
     * Sort tasks by last_updated timestamp in descending order.
     * If last_updated is undefined, sort by id in descend
     */
    .sort((a: Task, b: Task) => {
      if (a.last_updated === undefined || b.last_updated === undefined) {
        return b.id - a.id;
      }

      if (a.last_updated < b.last_updated) {
        return 1;
      } else if (a.last_updated > b.last_updated) {
        return -1;
      } else {
        return 0;
      }
    });

  return {
    lastViewedTaskTimestamp,
    tasks: filteredTasks,
    isLoading,
    markAllViewed: handleMarkAllViewed,
    markViewed: handleMarkViewed,
    allViewed,
    refetch,
    count: filteredTasks.length,
    toggleHidden: handleToggleHideViewed,
    hiddenTasks: hideViewed,
  };
};

export default useTasks;
