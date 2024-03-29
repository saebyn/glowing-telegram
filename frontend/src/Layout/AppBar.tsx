import { AppBar, AppBarClasses, TitlePortal } from "react-admin";
import { TwitchLoginButton } from "../twitch/LoginButton";

const MyAppBar = () => (
  <AppBar>
    <TitlePortal className={AppBarClasses.title} />

    <TwitchLoginButton />
  </AppBar>
);

export default MyAppBar;
