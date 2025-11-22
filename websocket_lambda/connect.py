import json
import os
import time
import boto3
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

def handler(event, context):
    logger.info(f"Connect event: {json.dumps(event)}")
    
    connection_id = event.get('requestContext', {}).get('connectionId')
    if not connection_id:
        logger.error("No connectionId found in event")
        return {'statusCode': 400, 'body': 'No connectionId provided'}
    
    # Extract auth information from the authorizer context
    authorizer = event.get('requestContext', {}).get('authorizer', {})
    user_id = authorizer.get('userId', '')
    auth_type = authorizer.get('authType', 'FullAccess')
    widget_id = authorizer.get('widgetId', '')
    
    if not user_id and not widget_id:
        logger.error("No userId or widgetId found in authorizer context")
        return {'statusCode': 401, 'body': 'Unauthorized'}
    
    try:
        ttl = int(time.time()) + (24 * 60 * 60)  # 24 hours in seconds
        
        item = {
            'connectionId': connection_id,
            'ttl': ttl,
            'connected_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
            'authType': auth_type
        }
        
        if user_id:
            item['user_id'] = user_id
        if widget_id:
            item['widgetId'] = widget_id
        
        connections_table.put_item(Item=item)
        
        logger.info(f"Connection {connection_id} stored successfully (authType: {auth_type})")
        return {'statusCode': 200, 'body': 'Connected'}
    
    except Exception as e:
        logger.error(f"Error storing connection: {str(e)}")
        return {'statusCode': 500, 'body': 'Failed to connect'}
