import { useGetList, useStore } from "react-admin";

const useTasks = () => {
  const [hideViewed, setHideViewed] = useStore("hideViewedTasks", false);

  const [lastViewedTaskTimestamp, setLastViewedTaskTimestamp] = useStore(
    "lastViewedTaskTimestamp",
    ""
  );

  const { data: tasks, refetch, isLoading } = useGetList("tasks");

  const handleToggleHideViewed = () => {
    setHideViewed((hideViewed) => !hideViewed);
  };

  const handleMarkAllViewed = () => {
    if (tasks && tasks.length > 0) {
      setLastViewedTaskTimestamp(tasks[0].last_updated);
    }
  };

  const handleMarkViewed = (taskId: string) => {
    if (tasks) {
      const task = tasks.find((t) => t.id === taskId);
      if (task) {
        setLastViewedTaskTimestamp(task.last_updated);
      }
    }
  };

  const allViewed = tasks
    ? tasks.every((task) => task.last_updated <= lastViewedTaskTimestamp)
    : false;

  const filteredTasks = (tasks || []).filter((task: any) =>
    hideViewed ? task.last_updated > lastViewedTaskTimestamp : true
  );

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
