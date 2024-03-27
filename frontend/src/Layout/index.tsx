import { ComponentProps } from "react";

import { Layout as AdminLayout } from "react-admin";

import AppBar from "./AppBar";

type LayoutProps = ComponentProps<typeof AdminLayout>;

const Layout = (props: LayoutProps) => {
  return <AdminLayout {...props} appBar={AppBar} />;
};

export default Layout;
