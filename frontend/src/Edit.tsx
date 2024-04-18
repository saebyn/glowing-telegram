/**
 * Customized react-admin Edit component to use instead of the default one.
 */
import {
  Edit as RAEdit,
  EditProps as RAEditProps,
  PrevNextButtons,
  TopToolbar,
} from "react-admin";

const Actions = () => (
  <TopToolbar>
    <PrevNextButtons />
  </TopToolbar>
);

const GTEdit: typeof RAEdit = ({ children, ...props }) => (
  <RAEdit redirect={false} actions={<Actions />} {...props}>
    {children}
  </RAEdit>
);

GTEdit.propTypes = RAEdit.propTypes;

export type EditProps = RAEditProps;

export default GTEdit;
