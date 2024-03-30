import { FunctionField } from "react-admin";

const ThumbnailField = ({ width, height, source }: any) => {
  const widthValue = width || 100;
  const heightValue = height || 100;

  return (
    <FunctionField
      sortable={false}
      render={(record: any) => {
        if (!record[source]) {
          return null;
        }

        const thumbnailUrl = record[source]
          .replace("%{width}", widthValue)
          .replace("%{height}", heightValue);
        return (
          <img
            src={thumbnailUrl}
            alt={record.title}
            width={widthValue}
            height={heightValue}
          />
        );
      }}
    />
  );
};

export default ThumbnailField;
