import simpleRestDataProvider from "ra-data-simple-rest";

export const dataProvider = simpleRestDataProvider(
  process.env.API_HOST || "http://localhost:3000/records"
);
