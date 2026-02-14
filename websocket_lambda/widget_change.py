import json
import os
import boto3
import logging
from botocore.exceptions import ClientError
from utils import deserialize_dynamodb_item, paginated_query, decimal_default

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource("dynamodb")
connections_table = dynamodb.Table(os.environ["CONNECTIONS_TABLE"])

# Initialize API Gateway Management API client
endpoint = os.environ.get("WEBSOCKET_ENDPOINT", "")
if endpoint.startswith("wss://"):
    endpoint = endpoint.replace("wss://", "https://")
api_client = (
    boto3.client("apigatewaymanagementapi", endpoint_url=endpoint) if endpoint else None
)


def handler(event, context):
    logger.info(f"Widget change event received")

    if not api_client:
        logger.error(
            f"No valid WebSocket endpoint configured: {os.environ.get('WEBSOCKET_ENDPOINT')}"
        )
        return

    # Process each record in the DynamoDB stream
    for record in event.get("Records", []):
        if record.get("eventName") in ["MODIFY", "INSERT"]:
            try:
                # Extract the new and old images
                if "dynamodb" in record and "NewImage" in record["dynamodb"]:
                    new_widget = deserialize_dynamodb_item(
                        record["dynamodb"]["NewImage"]
                    )
                    old_widget = deserialize_dynamodb_item(
                        record["dynamodb"].get("OldImage")
                    )
                    handle_widget_change(
                        new_widget, old_widget, record.get("eventName")
                    )
            except Exception as e:
                logger.error(f"Error processing record: {str(e)}")


def handle_widget_change(widget, old_widget, event_name):
    if not widget or "id" not in widget:
        logger.warning("No valid widget data found in the record")
        return

    widget_id = widget.get("id")

    # Check if config changed (for WIDGET_CONFIG_UPDATE)
    # For INSERT events, always send initial config
    config_changed = False
    if event_name == "INSERT":
        config_changed = True
    elif event_name == "MODIFY" and old_widget:
        config_changed = widget.get("config") != old_widget.get("config")

    # Check if state changed (for WIDGET_STATE_UPDATE)
    # For INSERT events, always send initial state
    state_changed = False
    if event_name == "INSERT":
        state_changed = True
    elif event_name == "MODIFY" and old_widget:
        state_changed = widget.get("state") != old_widget.get("state")

    # Find connections subscribed to this widget
    user_id = widget.get("user_id")
    if not user_id:
        logger.warning(f"No user_id found for widget: {widget_id}")
        return

    try:
        # Find all connections for this user that are subscribed to this widget
        connections = find_connections_for_widget(user_id, widget_id)

        if not connections:
            logger.info(f"No active connections subscribed to widget {widget_id}")
            return

        # Broadcast config update if config changed
        if config_changed:
            config_message = json.dumps(
                {
                    "type": "WIDGET_CONFIG_UPDATE",
                    "widgetId": widget_id,
                    "config": widget.get("config", {}),
                },
                default=decimal_default,
            )
            broadcast_to_connections(connections, config_message)

        # Broadcast state update if state changed
        if state_changed:
            state_message = json.dumps(
                {
                    "type": "WIDGET_STATE_UPDATE",
                    "widgetId": widget_id,
                    "state": widget.get("state", {}),
                    "timestamp": widget.get("updated_at", ""),
                },
                default=decimal_default,
            )
            broadcast_to_connections(connections, state_message)

    except Exception as e:
        logger.error(f"Error processing widget change: {str(e)}")


def find_connections_for_widget(user_id: str | None, widget_id: str):
    """Find all connections for a user that are subscribed to a specific widget

    This includes both:
    - Connections with FullAccess (user_id based) that subscribed to the widget
    - Connections with WidgetAccess (widget token) for this specific widget
    """
    try:
        subscribed_connections = set()

        # Find user connections that have subscribed to this widget
        if user_id:
            for item in paginated_query(
                connections_table,
                IndexName="user_id-index",
                KeyConditionExpression="user_id = :userId",
                FilterExpression="contains(subscribed_widgets, :widgetId)",
                ExpressionAttributeValues={":userId": user_id, ":widgetId": widget_id},
            ):
                subscribed_connections.add(item["connectionId"])

        # Also find WidgetAccess connections for this specific widget
        # These connections are authenticated with a widget token and automatically
        # receive updates for that widget - no explicit subscription required
        for item in paginated_query(
            connections_table,
            IndexName="widgetId-index",
            KeyConditionExpression="widgetId = :widgetId",
            ExpressionAttributeValues={":widgetId": widget_id},
        ):
            subscribed_connections.add(item["connectionId"])

        return subscribed_connections

    except Exception as e:
        logger.error(f"Error querying connections for widget: {str(e)}")
        return []


def broadcast_to_connections(connections, message):
    if not api_client:
        logger.error(
            f"No valid WebSocket endpoint configured: {os.environ.get('WEBSOCKET_ENDPOINT')}"
        )
        return

    for connection_id in connections:
        try:
            api_client.post_to_connection(ConnectionId=connection_id, Data=message)
            logger.info(f"Successfully sent update to connection {connection_id}")
        except ClientError as e:
            # Handle stale connections
            if e.response.get("Error", {}).get("Code") == "GoneException":
                logger.info(f"Connection {connection_id} is stale, removing it")
                remove_connection(connection_id)
            else:
                logger.error(
                    f"Error sending message to connection {connection_id}: {str(e)}"
                )


def remove_connection(connection_id):
    try:
        connections_table.delete_item(Key={"connectionId": connection_id})
    except Exception as e:
        logger.error(f"Error removing stale connection {connection_id}: {str(e)}")
