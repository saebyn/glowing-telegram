import json
import time
import os
import boto3
from botocore.exceptions import ClientError

# In-memory cache for version config
version_cache = {
    'version': None,
    'timestamp': 0,
    'ttl': 60000  # 60 seconds in milliseconds
}

# Configuration key constant
CONFIG_KEY = 'config/version.json'

def get_bucket_name():
    """Get bucket name from config"""
    try:
        import config
        return config.BUCKET_NAME
    except ImportError:
        # Fallback for testing or if config is not generated
        return os.environ.get('BUCKET_NAME')

def get_fallback_version():
    """Get fallback version from config"""
    try:
        import config
        return config.FALLBACK_VERSION
    except ImportError:
        # Fallback for testing or if config is not generated
        return os.environ.get('FALLBACK_VERSION')

def handler(event, context):
    """
    Lambda@Edge function to dynamically select frontend version
    This function intercepts CloudFront requests and rewrites the origin path
    based on the version specified in S3 config/version.json
    """
    request = event['Records'][0]['cf']['request']
    
    try:
        # Get current version from cache or S3
        current_version = get_current_version()
        
        # Use fallback version if no version found from S3
        fallback_version = get_fallback_version()
        version_to_use = current_version or fallback_version
        
        if not version_to_use:
            print('No version available (neither from S3 nor fallback), proceeding with original request')
            return request
        
        # Rewrite the origin path to include the version
        original_uri = request['uri']
        
        # Handle root path requests
        if original_uri == '/' or original_uri == '':
            request['uri'] = f'/{version_to_use}/index.html'
        else:
            # Prepend version to the path
            request['uri'] = f'/{version_to_use}{original_uri}'
        
        print(f'Rewritten URI from {original_uri} to {request["uri"]} for version {version_to_use}')
        
    except Exception as error:
        print(f'Error in version selector: {error}')
        # Use fallback version on error to maintain availability
        fallback_uri = request['uri']
        fallback_version = get_fallback_version()
        
        if fallback_version:
            if fallback_uri == '/' or fallback_uri == '':
                request['uri'] = f'/{fallback_version}/index.html'
            else:
                request['uri'] = f'/{fallback_version}{fallback_uri}'
            print(f'Using fallback version {fallback_version} due to error')
    
    return request

def get_current_version():
    """
    Get current version with caching
    """
    bucket_name = get_bucket_name()
    now = int(time.time() * 1000)  # Current time in milliseconds
    
    # Return cached version if still valid
    if version_cache['version'] and (now - version_cache['timestamp']) < version_cache['ttl']:
        print(f'Using cached version: {version_cache["version"]}')
        return version_cache['version']
    
    try:
        # Fetch new version from S3
        s3 = boto3.client('s3', region_name='us-east-1')  # Lambda@Edge requires us-east-1
        
        response = s3.get_object(Bucket=bucket_name, Key=CONFIG_KEY)
        config_data = json.loads(response['Body'].read().decode('utf-8'))
        
        if config_data.get('version'):
            # Update cache
            version_cache['version'] = config_data['version']
            version_cache['timestamp'] = now
            
            print(f'Fetched and cached new version: {config_data["version"]}')
            return config_data['version']
        else:
            print('Version not found in config file')
            return None
            
    except ClientError as error:
        print(f'Error fetching version from S3: {error}')
        
        # Return cached version if available, even if expired
        if version_cache['version']:
            print(f'Using stale cached version due to S3 error: {version_cache["version"]}')
            return version_cache['version']
        
        return None
    except Exception as error:
        print(f'Unexpected error fetching version from S3: {error}')
        
        # Return cached version if available, even if expired
        if version_cache['version']:
            print(f'Using stale cached version due to unexpected error: {version_cache["version"]}')
            return version_cache['version']
        
        return None

def reset_cache():
    """
    Reset cache for testing purposes
    """
    version_cache['version'] = None
    version_cache['timestamp'] = 0