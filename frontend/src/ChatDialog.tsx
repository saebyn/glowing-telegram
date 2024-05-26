import { useState } from "react";

import Dialog from "@mui/material/Dialog";
import DialogActions from "@mui/material/DialogActions";
import DialogContent from "@mui/material/DialogContent";
import DialogTitle from "@mui/material/DialogTitle";
import TextField from "@mui/material/TextField";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import CardHeader from "@mui/material/CardHeader";
import CardActions from "@mui/material/CardActions";
import Typography from "@mui/material/Typography";
import { IconButtonProps } from "@mui/material/IconButton";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";

import { ChatMessage } from "./types";
import styled from "@mui/material/styles/styled";
import IconButton from "@mui/material/IconButton";
import Collapse from "@mui/material/Collapse";

interface ChatDialogProps {
  open: boolean;
  job: string;
  transcript: string;
  context: string;
  onChat: (_messages: ChatMessage[]) => Promise<ChatMessage[]>;
  onChange: (_content: string) => void;
  onClose: () => void;
}

function ChatDialog({
  open,
  job,
  transcript,
  context,
  onChat: sendMessages,
  onChange,
  onClose,
}: ChatDialogProps) {
  const [loading, setLoading] = useState<boolean>(false);
  const [messageContent, setMessageContent] = useState<string>("");

  const baseMessages: ChatMessage[] = [
    {
      role: "system",
      content: job,
    },
    {
      role: "user",
      content: `Video context:\n${context}`,
    },
    {
      role: "user",
      content: transcript,
    },
  ];

  const [messages, setMessages] = useState<ChatMessage[]>(baseMessages);

  const chatMessageStartIndex = baseMessages.length;
  const chatMessages = messages.slice(chatMessageStartIndex);

  const handleClear = () => {
    setMessages(() => baseMessages);
  };

  return (
    <Dialog open={open} onClose={onClose} fullWidth={true} maxWidth="lg">
      <DialogTitle>ChatDialog</DialogTitle>

      <DialogContent>
        {baseMessages.map((message, index) => (
          <ChatMessageView key={index} message={message} disabled={true} />
        ))}

        {chatMessages.map((message, index) => (
          <ChatMessageView
            key={index + chatMessageStartIndex}
            message={message}
            onChange={onChange}
          />
        ))}

        <hr />

        {loading && <p>Loading...</p>}
      </DialogContent>

      <DialogActions>
        <form
          onSubmit={async (event) => {
            event.preventDefault();
            setLoading(true);
            const result = await sendMessages([
              ...messages,
              {
                role: "user",
                content: messageContent,
              },
            ]);
            setLoading(false);
            setMessages(result);
            setMessageContent("");
          }}
        >
          {messages.length > chatMessageStartIndex && (
            <TextField
              disabled={loading}
              name="message"
              placeholder="Enter a message..."
              multiline
              rows={2}
              value={messageContent}
              onChange={(event) => {
                setMessageContent(event.target.value);
              }}
            />
          )}
          <Button color="primary" type="submit">
            {loading
              ? "Sending..."
              : messages.length === chatMessageStartIndex
              ? "Start"
              : "Send"}
          </Button>

          <Button color="secondary" onClick={onClose}>
            Close
          </Button>

          <Button color="secondary" onClick={handleClear}>
            Clear
          </Button>
        </form>
      </DialogActions>
    </Dialog>
  );
}

const ChatMessageView = ({
  message,
  disabled,
  onChange,
  ...props
}: {
  message: ChatMessage;
  disabled?: boolean;
  onChange?: (_content: string) => void;
  [key: string]: any;
}) => {
  const [expanded, setExpanded] = useState<boolean>(false);

  const handleExpandClick = () => {
    setExpanded(!expanded);
  };

  return (
    <Card {...props}>
      <CardHeader
        title={message.role}
        subheader={
          expanded
            ? null
            : message.content.substring(0, 50) +
              (message.content.length > 50 ? "..." : "")
        }
      />
      <Collapse in={expanded} timeout="auto" unmountOnExit>
        <CardContent>
          <Typography variant="caption">
            <pre>{message.content}</pre>
          </Typography>
        </CardContent>
      </Collapse>

      <CardActions>
        {!disabled && onChange && message.role === "assistant" && (
          <Button
            variant="contained"
            onClick={() => {
              onChange(message.content);
            }}
          >
            Use this message
          </Button>
        )}

        <ExpandMore
          expand={expanded}
          onClick={handleExpandClick}
          aria-expanded={expanded}
          aria-label="show more"
        >
          <ExpandMoreIcon />
        </ExpandMore>
      </CardActions>
    </Card>
  );
};

interface ExpandMoreProps extends IconButtonProps {
  expand: boolean;
}

const ExpandMore = styled((props: ExpandMoreProps) => {
  const { expand: _, ...other } = props;
  return <IconButton {...other} />;
})(({ theme, expand }) => ({
  transform: !expand ? "rotate(0deg)" : "rotate(180deg)",
  marginLeft: "auto",
  transition: theme.transitions.create("transform", {
    duration: theme.transitions.duration.shortest,
  }),
}));

export default ChatDialog;
