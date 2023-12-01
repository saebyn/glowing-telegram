import simpleRestDataProvider from "ra-data-simple-rest";

const baseUrl = `${import.meta.env.VITE_API_URL || "http://localhost:3000"}`;

const baseDataProvider = simpleRestDataProvider(`${baseUrl}/records`);

export const dataProvider = {
  ...baseDataProvider,

  // custom methods

  getStreamClips: async (prefix: string) => {
    const url = new URL(`${baseUrl}/stream_ingestion/find_files`);
    url.searchParams.append("prefix", prefix);

    return fetch(url).then((res) => res.json());
  },
};
