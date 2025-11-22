import json
import os
import jwt
import logging
import boto3
import uuid

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Cognito User Pool details
USER_POOL_ID = os.environ["USER_POOL_ID"]
CLIENT_ID = os.environ["USER_POOL_CLIENT_ID"]
REGION = os.environ.get("AWS_REGION", "us-west-2")
STREAM_WIDGETS_TABLE = os.environ.get("STREAM_WIDGETS_TABLE", "")

keys_url = (
    f"https://cognito-idp.{REGION}.amazonaws.com/{USER_POOL_ID}/.well-known/jwks.json"
)

# Initialize the PyJWKClient with the keys URL
jwks_client = jwt.PyJWKClient(keys_url)

# Initialize DynamoDB client
dynamodb = boto3.resource('dynamodb')


def is_valid_uuid(token):
    """Check if token is a valid UUID format"""
    try:
        uuid.UUID(token)
        return True
    except (ValueError, AttributeError):
        return False


def verify_widget_token(token):
    """Verify widget access token by looking it up in DynamoDB"""
    if not STREAM_WIDGETS_TABLE:
        logger.error("STREAM_WIDGETS_TABLE environment variable not set")
        return None
    
    if not is_valid_uuid(token):
        logger.info("Token is not a valid UUID, not a widget token")
        return None
    
    try:
        table = dynamodb.Table(STREAM_WIDGETS_TABLE)
        response = table.query(
            IndexName='access_token-index',
            KeyConditionExpression='access_token = :token',
            ExpressionAttributeValues={
                ':token': token
            },
            Limit=1
        )
        
        items = response.get('Items', [])
        if items:
            widget = items[0]
            logger.info(f"Widget token valid for widget: {widget['id']}")
            return {
                'widgetId': widget['id'],
                'userId': widget.get('user_id'),
                'authType': 'WidgetAccess'
            }
        else:
            logger.warning("Widget token not found in database")
            return None
    except Exception as e:
        logger.exception(f"Error verifying widget token: {str(e)}")
        return None


def verify_cognito_jwt(token):
    """Verify Cognito JWT token"""
    try:
        # Get the JWT header to extract the key ID (kid)
        jwt_headers = jwt.get_unverified_header(token)
        kid = jwt_headers["kid"]

        # Fetch the public keys from the JWKS endpoint
        key = jwks_client.get_signing_key(kid)

        if key is None:
            logger.warning(f"No matching key found for kid: {kid}")
            return None

        # Verify the token
        payload = jwt.decode(
            token,
            key=key,
            algorithms=["RS256"],
            options={"verify_signature": True, "verify_exp": True, "verify_aud": True},
            audience=CLIENT_ID,
        )

        # Extract user information
        user_id = payload["sub"]
        email = payload.get("email", "")

        logger.info(f"Cognito JWT is valid for user {user_id}")

        return {
            'userId': user_id,
            'email': email,
            'authType': 'FullAccess'
        }

    except Exception as e:
        logger.exception(f"Error validating Cognito JWT: {str(e)}")
        return None


def handler(event, context):
    logger.info(f"Event: {json.dumps(event)}")

    # Extract the token parameter from the event
    token = event.get("queryStringParameters", {}).get("token")

    if not token:
        logger.warning("No token provided in query string parameters.")
        return generate_policy("user", "Deny", event["methodArn"])

    token = token.replace("Bearer ", "")

    # Try to verify as Cognito JWT first
    cognito_auth = verify_cognito_jwt(token)
    if cognito_auth:
        return generate_policy(
            cognito_auth['userId'], 
            "Allow", 
            event["methodArn"], 
            {
                "userId": cognito_auth['userId'], 
                "email": cognito_auth.get('email', ''),
                "authType": cognito_auth['authType']
            }
        )

    # If not a Cognito JWT, try widget token
    widget_auth = verify_widget_token(token)
    if widget_auth:
        return generate_policy(
            f"widget-{widget_auth['widgetId']}", 
            "Allow", 
            event["methodArn"], 
            {
                "widgetId": widget_auth['widgetId'],
                "userId": widget_auth.get('userId', ''),
                "authType": widget_auth['authType']
            }
        )

    # Both authentication methods failed
    logger.warning("Token validation failed for both Cognito JWT and widget token")
    return generate_policy("user", "Deny", event["methodArn"])


def generate_policy(principal_id, effect, resource, context=None):
    policy = {
        "principalId": principal_id,
        "policyDocument": {
            "Version": "2012-10-17",
            "Statement": [
                {"Action": "execute-api:Invoke", "Effect": effect, "Resource": resource}
            ],
        },
    }

    if context:
        policy["context"] = context

    return policy
