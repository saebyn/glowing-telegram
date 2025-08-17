import json
import os
import boto3
from datetime import datetime, timedelta, timezone
import urllib.request
import urllib.parse


def check_youtube_auth_valid(user_id):
    """Check if user has valid YouTube refresh token by testing it against Google OAuth."""
    try:
        secrets_client = boto3.client('secretsmanager')
        
        # Get YouTube app credentials
        youtube_secret_arn = os.environ['YOUTUBE_SECRET_ARN']
        app_secret_response = secrets_client.get_secret_value(SecretId=youtube_secret_arn)
        app_credentials = json.loads(app_secret_response['SecretString'])
        
        client_id = app_credentials.get('client_id')
        client_secret = app_credentials.get('client_secret')
        
        if not client_id or not client_secret:
            print(f"YouTube app credentials missing for user {user_id}")
            return False
        
        # Get user's YouTube session secret
        user_secret_path = f"{os.environ['USER_SECRET_PATH']}/{user_id}"
        response = secrets_client.get_secret_value(SecretId=user_secret_path)
        
        # Parse the secret
        secret_data = json.loads(response['SecretString'])
        
        # Check if we have refresh token
        refresh_token = secret_data.get('refresh_token')
        
        if not refresh_token:
            return False
        
        # Test the refresh token by actually using it to get an access token
        token_data = urllib.parse.urlencode({
            'client_id': client_id,
            'client_secret': client_secret,
            'refresh_token': refresh_token,
            'grant_type': 'refresh_token'
        }).encode('utf-8')
        
        req = urllib.request.Request(
            'https://oauth2.googleapis.com/token',
            data=token_data,
            headers={'Content-Type': 'application/x-www-form-urlencoded'}
        )
        
        try:
            with urllib.request.urlopen(req) as response:
                if response.status == 200:
                    # Successfully refreshed token, refresh token is valid
                    return True
                else:
                    print(f"Token refresh failed with status {response.status} for user {user_id}")
                    return False
        except urllib.error.HTTPError as e:
            print(f"Token refresh failed with HTTP error {e.code} for user {user_id}")
            return False
        
    except Exception as e:
        print(f"YouTube auth check failed for user {user_id}: {str(e)}")
        return False


def handler(event, context):
    # Resource setup
    dynamodb = boto3.resource('dynamodb')
    sfn = boto3.client('stepfunctions')
    events = boto3.client('events')
    table = dynamodb.Table(os.environ['EPISODES_TABLE_NAME'])
    now = datetime.now(timezone.utc).isoformat()

    # Parse the event
    try:
        claims = event['requestContext']['authorizer']['jwt']['claims']
        user_id = claims['sub']
    except (KeyError, TypeError):
        return {
            'statusCode': 401,
            'body': 'Unauthorized',
        }

    try:
        request_body = json.loads(event['body'])
        episode_ids = request_body.get('episode_ids', [])
    except (KeyError, json.JSONDecodeError):
        return {
            'statusCode': 400,
            'body': 'Invalid request format',
        }

    # Validate the input
    if not episode_ids:
        return {
            'statusCode': 400,
            'body': 'No episode IDs provided',
        }
    if not all(isinstance(episode_id, str) for episode_id in episode_ids):
        return {
            'statusCode': 400,
            'body': 'Invalid episode IDs provided',
        }
    if not user_id:
        return {
            'statusCode': 401,
            'body': 'Unauthorized',
        }

    # Check YouTube authentication before queuing episodes
    if not check_youtube_auth_valid(user_id):
        return {
            'statusCode': 403,
            'body': json.dumps({
                'error': 'YouTube authentication required',
                'message': 'Please authorize YouTube access before queuing episodes for upload.'
            }),
        }

    # Update the records
    for episode_id in episode_ids:
        table.update_item(
            Key={'id': episode_id},
            UpdateExpression='SET #userId = :userId, #uploadStatus = :uploadStatus, #updatedAt = :now, #uploadQueueTimestamp = :now',
            ExpressionAttributeNames={'#userId': 'user_id', '#uploadStatus': 'upload_status', '#updatedAt': 'updated_at', '#uploadQueueTimestamp': 'upload_queue_timestamp'},
            ExpressionAttributeValues={':userId': user_id, ':uploadStatus': os.environ['UPLOAD_READY_TO_UPLOAD'], ':now': now},
        )

    # Send events
    events.put_events(
        Entries=[
            {
                'Source': 'glowing-telegram.youtube-uploader',
                'DetailType': 'EpisodeUploadStatus',
                'Detail': json.dumps({
                    'status': 'PENDING',
                    'episodeId': episode_id,
                    'userId': user_id,
                }),
                'EventBusName': os.environ['EVENT_BUS_NAME'],
            }
            for episode_id in episode_ids
        ]
    )

    # Check if the step function is already running
    response = sfn.list_executions(
        stateMachineArn=os.environ['STEPFUNCTION_ARN'],
        statusFilter='RUNNING'
    )
    if not response['executions']:
        # Start the step function execution if it's not running
        sfn.start_execution(
            stateMachineArn=os.environ['STEPFUNCTION_ARN'],
            input='{}'
        )

    return {
        'statusCode': 200,
        'body': '',
    }