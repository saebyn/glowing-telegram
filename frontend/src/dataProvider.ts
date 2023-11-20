import simpleRestDataProvider from "ra-data-simple-rest";

export const dataProvider = simpleRestDataProvider(
  `${import.meta.env.VITE_API_URL || "http://localhost:3000"}/records`
);
