import {
  Admin,
  Resource,
  ListGuesser,
  EditGuesser,
  ShowGuesser,
  CustomRoutes,
} from "react-admin";
import { Route, BrowserRouter } from "react-router-dom";

import { TwitchLoginPage } from "./twitchLogin/Page";
import { dataProvider } from "./dataProvider";

import streamViews from "./resources/streams";
import videoClipsViews from "./resources/videoClips";
import episodeViews from "./resources/episodes";

import Layout from "./Layout";

export const App = () => (
  <BrowserRouter>
    <Admin dataProvider={dataProvider} layout={Layout}>
      <Resource name="video_clips" {...videoClipsViews} />
      <Resource name="streams" {...streamViews} />
      <Resource name="episodes" {...episodeViews} />
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

      <CustomRoutes>
        <Route path="/twitch/callback" element={<TwitchLoginPage />} />
      </CustomRoutes>
    </Admin>
  </BrowserRouter>
);
