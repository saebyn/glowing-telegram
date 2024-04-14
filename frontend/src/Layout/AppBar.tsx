import { AppBar, AppBarClasses, TitlePortal } from "react-admin";
import { TwitchLoginButton } from "../twitch/LoginButton";
import { YoutubeLoginButton } from "../youtube/LoginButton";
import TasksDrawer from "../TasksDrawer";

const MyAppBar = () => (
  <AppBar>
    <TitlePortal className={AppBarClasses.title} />

    <TwitchLoginButton />
    <YoutubeLoginButton />

    <TasksDrawer />
  </AppBar>
);

export default MyAppBar;
