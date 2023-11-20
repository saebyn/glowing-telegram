import {
  Admin,
  Resource,
  ListGuesser,
  EditGuesser,
  ShowGuesser,
} from "react-admin";
import { dataProvider } from "./dataProvider";

import streamViews from "./resources/streams";

export const App = () => (
  <Admin dataProvider={dataProvider}>
    <Resource
      name="video_clips"
      list={ListGuesser}
      edit={EditGuesser}
      show={ShowGuesser}
    />
    <Resource
      name="streams"
      list={streamViews.list}
      edit={streamViews.edit}
      show={streamViews.show}
      create={streamViews.create}
    />
    <Resource
      name="episodes"
      list={ListGuesser}
      edit={EditGuesser}
      show={ShowGuesser}
    />
    <Resource
      name="topics"
      list={ListGuesser}
      edit={EditGuesser}
      show={ShowGuesser}
    />
    <Resource
      name="transcriptions"
      list={ListGuesser}
      edit={EditGuesser}
      show={ShowGuesser}
    />
  </Admin>
);
