import {
  Admin,
  Resource,
  ListGuesser,
  EditGuesser,
  ShowGuesser,
} from "react-admin";
import { dataProvider } from "./dataProvider";
import StreamList from "./resources/streams/List";

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
      list={StreamList}
      edit={EditGuesser}
      show={ShowGuesser}
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
