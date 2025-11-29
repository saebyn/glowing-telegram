import json
import os
import boto3
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')
connections_table = dynamodb.Table(os.environ['CONNECTIONS_TABLE'])

def handler(event, context):
    logger.info(f"Disconnect event: {json.dumps(event)}")
    
    connection_id = event.get('requestContext', {}).get('connectionId')
    if not connection_id:
        logger.error("No connectionId found in event")
        return {'statusCode': 400, 'body': 'No connectionId provided'}
    
    try:
        # Remove the connection from DynamoDB
        connections_table.delete_item(
            Key={
                'connectionId': connection_id
            }
        )
        
        logger.info(f"Connection {connection_id} removed successfully")
        return {'statusCode': 200, 'body': 'Disconnected'}
    
    except Exception as e:
        logger.error(f"Error removing connection: {str(e)}")
        return {'statusCode': 500, 'body': 'Failed to disconnect'}
