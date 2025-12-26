import json
import os
import boto3
import logging
from datetime import datetime, timezone

from botocore.exceptions import ClientError
from utils import decimal_default

logger = logging.getLogger()
logger.setLevel(logging.INFO)


# Initialize clients
dynamodb = boto3.resource("dynamodb")
connections_table = dynamodb.Table(os.environ["CONNECTIONS_TABLE"])
widgets_table = dynamodb.Table(os.environ["STREAM_WIDGETS_TABLE"])

# Initialize API Gateway Management API client
endpoint = os.environ.get("WEBSOCKET_ENDPOINT", "")
if not endpoint:
    logger.warning("WEBSOCKET_ENDPOINT environment variable not set")
elif endpoint.startswith("wss://"):
    endpoint = endpoint.replace("wss://", "https://")

api_client = (
    boto3.client("apigatewaymanagementapi", endpoint_url=endpoint) if endpoint else None
)


def handler(event, context):
    logger.info(f"Message event: {json.dumps(event)}")

    connection_id = event.get("requestContext", {}).get("connectionId")
    if not connection_id:
        logger.error("No connectionId found")
        return {"statusCode": 400, "body": "Bad Request"}

    # Get connection info
    try:
        conn_response = connections_table.get_item(Key={"connectionId": connection_id})
        connection = conn_response.get("Item")
        if not connection:
            logger.error(f"Connection {connection_id} not found")
            return {"statusCode": 404, "body": "Connection not found"}
    except Exception as e:
        logger.error(f"Error fetching connection: {str(e)}")
        return {"statusCode": 500, "body": "Internal error"}

    # Parse message
    body = event.get("body", "{}")
    try:
        message = json.loads(body)
    except json.JSONDecodeError:
        logger.error("Invalid JSON in message body")
        return {"statusCode": 400, "body": "Invalid JSON"}

    message_type = message.get("type")

    if message_type == "WIDGET_SUBSCRIBE":
        return handle_subscribe(connection_id, connection, message)
    elif message_type == "WIDGET_UNSUBSCRIBE":
        return handle_unsubscribe(connection_id, message)
    elif message_type == "WIDGET_ACTION":
        return handle_action(connection_id, connection, message)
    else:
        logger.warning(f"Unknown message type: {message_type}")
        return {"statusCode": 400, "body": "Unknown message type"}


def handle_subscribe(connection_id: str, connection: dict, message: dict):
    widget_id = message.get("widgetId")
    if not widget_id:
        return {"statusCode": 400, "body": "Missing widgetId"}

    widget = None
    try:
        # Use get_item for direct lookup by partition key
        response = widgets_table.get_item(Key={"id": widget_id})
        widget = response.get("Item")

        if not widget:
            return {"statusCode": 404, "body": "Widget not found"}
    except Exception as e:
        logger.error(f"Error fetching widget: {str(e)}")
        return {"statusCode": 500, "body": "Internal error"}

    # Validate access
    auth_type = connection.get("authType", "FullAccess")

    if auth_type == "WidgetAccess":
        # Widget token can only access its own widget
        if connection.get("widgetId") != widget_id:
            logger.warning(
                f"Widget access denied: {connection.get('widgetId')} != {widget_id}"
            )
            return {"statusCode": 403, "body": "Forbidden"}
    elif auth_type == "FullAccess":
        # User JWT can access any widget they own
        try:
            if widget.get("user_id") != connection.get("user_id"):
                logger.warning(
                    f"User {connection.get('user_id')} does not own widget {widget_id}"
                )
                return {"statusCode": 403, "body": "Forbidden"}
        except Exception as e:
            logger.error(f"Error fetching widget: {str(e)}")
            return {"statusCode": 500, "body": "Internal error"}

    # Add widget_id to the connection's subscribed_widgets set
    try:
        connections_table.update_item(
            Key={"connectionId": connection_id},
            UpdateExpression="ADD subscribed_widgets :widget_id",
            ExpressionAttributeValues={":widget_id": {widget_id}},
        )
        logger.info(f"Connection {connection_id} subscribed to widget {widget_id}")
    except Exception as e:
        logger.error(f"Error updating subscription: {str(e)}")
        return {"statusCode": 500, "body": "Failed to subscribe"}

    # Fetch and send initial state
    try:
        send_message(
            connection_id,
            {
                "type": "WIDGET_INITIAL_STATE",
                "widgetId": widget_id,
                "widget": widget,
            },
        )
    except Exception as e:
        logger.error(f"Error sending initial state: {str(e)}")

    return {"statusCode": 200, "body": "Subscribed"}


def handle_unsubscribe(connection_id: str, message: dict):
    widget_id = message.get("widgetId")
    if not widget_id:
        return {"statusCode": 400, "body": "Missing widgetId"}

    # Remove widget_id from the connection's subscribed_widgets set
    try:
        connections_table.update_item(
            Key={"connectionId": connection_id},
            UpdateExpression="DELETE subscribed_widgets :widget_id",
            ExpressionAttributeValues={":widget_id": {widget_id}},
        )
        logger.info(f"Connection {connection_id} unsubscribed from widget {widget_id}")
    except Exception as e:
        logger.error(f"Error removing subscription: {str(e)}")
        return {"statusCode": 500, "body": "Failed to unsubscribe"}

    return {"statusCode": 200, "body": "Unsubscribed"}


def handle_action(connection_id: str, connection: dict, message: dict):
    # Widget access is read-only
    auth_type = connection.get("authType", "FullAccess")
    if auth_type == "WidgetAccess":
        return {"statusCode": 403, "body": "Read-only access"}

    widget_id = message.get("widgetId")
    action = message.get("action")

    if not widget_id or not action:
        return {"statusCode": 400, "body": "Missing widgetId or action"}

    # Verify ownership
    try:
        response = widgets_table.get_item(Key={"id": widget_id})
        widget = response.get("Item")
        if not widget:
            return {"statusCode": 404, "body": "Widget not found"}

        if widget.get("user_id") != connection.get("user_id"):
            return {"statusCode": 403, "body": "Forbidden"}

        # Route to type-specific action handler
        widget_type = widget.get("type", "unknown")
        payload = message.get("payload", {})

        if widget_type == "countdown":
            result = handle_countdown_action(widget, action, payload)
        else:
            result = {"success": False, "error": f"Unknown widget type: {widget_type}"}

        if result["success"]:
            # Update widget state in DynamoDB
            widgets_table.update_item(
                Key={"id": widget_id},
                UpdateExpression="SET #state = :state, updated_at = :now",
                ExpressionAttributeNames={"#state": "state"},
                ExpressionAttributeValues={
                    ":state": result["new_state"],
                    ":now": datetime.now(timezone.utc).isoformat(),
                },
            )

        send_message(
            connection_id,
            {
                "type": "WIDGET_ACTION_RESPONSE",
                "widgetId": widget_id,
                "action": action,
                "success": result["success"],
                "result": result.get("data"),
                "error": result.get("error"),
            },
        )

        return {
            "statusCode": 200 if result["success"] else 400,
            "body": "Action executed",
        }
    except Exception as e:
        logger.error(f"Error executing action: {str(e)}")
        send_message(
            connection_id,
            {
                "type": "WIDGET_ACTION_RESPONSE",
                "widgetId": widget_id,
                "action": action,
                "success": False,
                "error": str(e),
            },
        )
        return {"statusCode": 500, "body": "Internal error"}


def handle_countdown_action(widget: dict, action: str, payload: dict) -> dict:
    """Handle countdown widget-specific actions"""
    state = widget.get("state", {})
    config = widget.get("config", {})

    if action == "start":
        # Start countdown with initial duration from config
        duration = config.get("duration", 300)  # Default 5 minutes
        return {
            "success": True,
            "new_state": {
                **state,
                "enabled": True,
                "last_tick_timestamp": datetime.now(timezone.utc).isoformat(),
                "duration_left": duration,
            },
        }

    elif action == "pause":
        # Pause countdown and update duration_left based on elapsed time
        current_duration = state.get("duration_left", 0)
        last_tick = state.get("last_tick_timestamp")

        if last_tick and state.get("enabled"):
            # Calculate elapsed time since last tick
            last_tick_dt = datetime.fromisoformat(last_tick)
            now = datetime.now(timezone.utc)
            elapsed_seconds = (now - last_tick_dt).total_seconds()

            # Update duration_left by subtracting elapsed time
            new_duration = max(0, current_duration - elapsed_seconds)
        else:
            # Already paused or no timestamp, keep current duration
            new_duration = current_duration

        return {
            "success": True,
            "new_state": {
                **state,
                "enabled": False,
                "duration_left": new_duration,
                "last_tick_timestamp": None,
            },
        }

    elif action == "resume":
        # Resume countdown from current duration_left
        return {
            "success": True,
            "new_state": {
                **state,
                "enabled": True,
                "last_tick_timestamp": datetime.now(timezone.utc).isoformat(),
            },
        }

    elif action == "reset":
        # Reset countdown to initial duration and stop
        # If currently running, we still reset to initial duration (no elapsed time matters)
        duration = config.get("duration", 300)
        # Intentionally not using **state to clear any ongoing countdown info
        return {
            "success": True,
            "new_state": {
                "enabled": False,
                "duration_left": duration,
                "last_tick_timestamp": None,
            },
        }

    elif action == "set_duration":
        # Update the duration to the provided value
        new_duration = payload.get("duration")
        if not isinstance(new_duration, (int, float)) or new_duration < 0:
            return {"success": False, "error": "Invalid duration value"}

        # If countdown is running, update last_tick_timestamp to now
        # so the new duration starts counting from this moment
        if state.get("enabled"):
            return {
                "success": True,
                "new_state": {
                    **state,
                    "duration_left": new_duration,
                    "last_tick_timestamp": datetime.now(timezone.utc).isoformat(),
                },
            }
        else:
            # Countdown is paused or stopped, just set the new duration
            return {
                "success": True,
                "new_state": {
                    **state,
                    "duration_left": new_duration,
                },
            }

    else:
        return {"success": False, "error": f"Unknown action: {action}"}


def send_message(connection_id: str, message: dict):
    if not api_client:
        logger.error("API Gateway Management API client not initialized")
        return

    try:
        api_client.post_to_connection(
            ConnectionId=connection_id,
            Data=json.dumps(message, default=decimal_default),
        )
    except ClientError as e:
        if e.response.get("Error", {}).get("Code") == "GoneException":
            logger.info(f"Connection {connection_id} is gone")
            # Clean up stale connection
            try:
                connections_table.delete_item(Key={"connectionId": connection_id})
            except Exception as cleanup_error:
                logger.error(f"Error cleaning up connection: {str(cleanup_error)}")
        else:
            logger.error(f"Error sending message: {str(e)}")
