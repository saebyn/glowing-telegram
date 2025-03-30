import json
import os
import jwt
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Cognito User Pool details
USER_POOL_ID = os.environ["USER_POOL_ID"]
CLIENT_ID = os.environ["USER_POOL_CLIENT_ID"]
REGION = os.environ.get("AWS_REGION", "us-west-2")

keys_url = (
    f"https://cognito-idp.{REGION}.amazonaws.com/{USER_POOL_ID}/.well-known/jwks.json"
)

# Initialize the PyJWKClient with the keys URL
jwks_client = jwt.PyJWKClient(keys_url)


def handler(event, context):
    logger.info(f"Event: {json.dumps(event)}")

    # Extract the token parameter from the event
    token = event.get("queryStringParameters", {}).get("token")

    if not token:
        logger.warning("No token provided in query string parameters.")
        return generate_policy("user", "Deny", event["methodArn"])

    token = token.replace("Bearer ", "")

    try:
        # Get the JWT header to extract the key ID (kid)
        jwt_headers = jwt.get_unverified_header(token)
        kid = jwt_headers["kid"]

        # Fetch the public keys from the JWKS endpoint
        key = jwks_client.get_signing_key(kid)

        if key is None:
            logger.warning(f"No matching key found for kid: {kid}")
            return generate_policy("user", "Deny", event["methodArn"])

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

        logger.info(f"Token is valid for user {user_id}")

        # Generate policy to allow the user to connect
        return generate_policy(
            user_id, "Allow", event["methodArn"], {"userId": user_id, "email": email}
        )

    except Exception as e:
        logger.exception(f"Error validating token: {str(e)}")
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
