import json
import os
import boto3
import logging
from botocore.exceptions import ClientError
from utils import deserialize_dynamodb_item, paginated_query

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

# Initialize API Gateway Management API client
endpoint = os.environ.get('WEBSOCKET_ENDPOINT', '')
if endpoint.startswith('wss://'):
    endpoint = endpoint.replace('wss://', 'https://')
    
api_client = boto3.client('apigatewaymanagementapi', endpoint_url=endpoint) if endpoint else None

def handler(event, context):
    logger.info(f"Task change event received")
    
    if not api_client:
        logger.error(f"No valid WebSocket endpoint configured: {os.environ.get('WEBSOCKET_ENDPOINT')}")
        return
    
    # Process each record in the DynamoDB stream
    for record in event.get('Records', []):
        if record.get('eventName') in ['MODIFY', 'INSERT']:
            try:
                # Extract the new image of the task
                if 'dynamodb' in record and 'NewImage' in record['dynamodb']:
                    new_task = deserialize_dynamodb_item(record['dynamodb']['NewImage'])
                    old_task = deserialize_dynamodb_item(record['dynamodb'].get('OldImage'))
                    handle_task_change(new_task, old_task)
            except Exception as e:
                logger.error(f"Error processing record: {str(e)}")
    
def handle_task_change(task, old_task):
    if not task or 'id' not in task:
        logger.warning("No valid task data found in the record")
        return
    
    # Get the user ID associated with this task
    user_id = task.get('user_id')
    if not user_id:
        logger.warning(f"No user_id found for task: {task.get('id')}")
        return
    
    try:
        # Find all active connections for this user
        connections = find_connections_for_user(user_id)
        
        if not connections:
            logger.info(f"No active connections found for user {user_id}")
            return
        
        # Prepare the payload to send to clients
        payload = json.dumps({
            'type': 'TASK_UPDATE',
            'task': task,
            'old_status': old_task.get('status') if old_task else None,
        })
        
        # Send the update to all connections for this user
        for connection_id in connections:
            try:
                api_client.post_to_connection(
                    ConnectionId=connection_id,
                    Data=payload
                )
                logger.info(f"Successfully sent update to connection {connection_id}")
            except ClientError as e:
                # Handle stale connections
                if e.response.get('Error', {}).get('Code') == 'GoneException':
                    logger.info(f"Connection {connection_id} is stale, removing it")
                    remove_connection(connection_id)
                else:
                    logger.error(f"Error sending message to connection {connection_id}: {str(e)}")
    
    except Exception as e:
        logger.error(f"Error processing task change: {str(e)}")

def find_connections_for_user(user_id):
    try:
        connections = [item.get('connectionId') 
                       for item in paginated_query(connections_table,
                                                   IndexName='user_id-index',
                                                   KeyConditionExpression='user_id = :userId',
                                                   ExpressionAttributeValues={':userId': user_id})]
        return connections
    
    except Exception as e:
        logger.error(f"Error querying connections for user: {str(e)}")
        return []

def remove_connection(connection_id):
    try:
        connections_table.delete_item(
            Key={
                'connectionId': connection_id
            }
        )
    except Exception as e:
        logger.error(f"Error removing stale connection {connection_id}: {str(e)}")
