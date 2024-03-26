import { Button, useDataProvider, useNotify, useRefresh } from "react-admin";
import { useMutation } from "react-query";

const TriggerRenderFileScanButton = () => {
  const dataProvider = useDataProvider();
  const { mutate, isLoading } = useMutation(() =>
    dataProvider.scanRenderFiles()
  );
  const notify = useNotify();
  const refresh = useRefresh();

  const handleClick = () => {
    mutate(void 0, {
      onSuccess: () => {
        notify("Scan complete", {
          type: "success",
        });

        refresh();
      },
      onError: () => {
        notify(`Error scanning files`, {
          type: "error",
        });
      },
    });
  };

  return (
    <Button label="Scan Files" onClick={handleClick} disabled={isLoading} />
  );
};

export default TriggerRenderFileScanButton;
