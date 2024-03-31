import { Admin, Resource, CustomRoutes } from "react-admin";
import { Route, BrowserRouter } from "react-router-dom";
import { LocalizationProvider } from "@mui/x-date-pickers";
import { AdapterDateFns } from "@mui/x-date-pickers/AdapterDateFns";
import { enUS } from "date-fns/locale";

import { TwitchLoginPage } from "./twitch/LoginPage";
import { dataProvider } from "./dataProvider";

import streamViews from "./resources/streams";
import videoClipsViews from "./resources/videoClips";
import episodeViews from "./resources/episodes";
import twitchStreamsViews from "./resources/twitchStreams";
import topicsViews from "./resources/topics";

import Layout from "./Layout";

export const App = () => (
  <BrowserRouter>
    <LocalizationProvider dateAdapter={AdapterDateFns} adapterLocale={enUS}>
      <Admin dataProvider={dataProvider} layout={Layout}>
        <Resource name="video_clips" {...videoClipsViews} />
        <Resource name="streams" {...streamViews} />
        <Resource name="episodes" {...episodeViews} />
        <Resource name="topics" {...topicsViews} />

        <Resource
          name="twitchStreams"
          {...twitchStreamsViews}
          options={{ label: "Twitch Import" }}
        />

        <CustomRoutes>
          <Route path="/twitch/callback" element={<TwitchLoginPage />} />
        </CustomRoutes>
      </Admin>
    </LocalizationProvider>
  </BrowserRouter>
);
