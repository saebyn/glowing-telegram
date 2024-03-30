import {
  Admin,
  Resource,
  ListGuesser,
  EditGuesser,
  ShowGuesser,
  CustomRoutes,
} from "react-admin";
import { Route, BrowserRouter } from "react-router-dom";

import { TwitchLoginPage } from "./twitch/LoginPage";
import { dataProvider } from "./dataProvider";

import streamViews from "./resources/streams";
import videoClipsViews from "./resources/videoClips";
import episodeViews from "./resources/episodes";
import twitchStreamsViews from "./resources/twitchStreams";

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

      <Resource
        name="twitchStreams"
        {...twitchStreamsViews}
        options={{ label: "Twitch Import" }}
      />

      <CustomRoutes>
        <Route path="/twitch/callback" element={<TwitchLoginPage />} />
      </CustomRoutes>
    </Admin>
  </BrowserRouter>
);
