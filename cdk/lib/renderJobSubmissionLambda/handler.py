import json
import boto3
import os
import datetime
import hashlib
import math

# Initialize clients lazily to avoid issues during testing
batch = None
dynamodb = None

def get_batch_client():
    global batch
    if batch is None:
        batch = boto3.client('batch')
    return batch

def get_dynamodb_client():
    global dynamodb
    if dynamodb is None:
        dynamodb = boto3.client('dynamodb')
    return dynamodb

# Maximum episodes per job to stay within ~20 GiB assumption
MAX_EPISODES_PER_JOB = int(os.environ.get('MAX_EPISODES_PER_JOB', '3'))

def split_episodes_into_chunks(episode_ids):
    """Split episode IDs into chunks that should fit within storage limits
    
    Args:
        episode_ids: List of episode ID strings
        
    Returns:
        List of lists, where each inner list contains episode IDs for one job
        
    Examples:
        >>> split_episodes_into_chunks(['ep1', 'ep2', 'ep3'])
        [['ep1', 'ep2', 'ep3']]
        
        >>> # Test with more than MAX_EPISODES_PER_JOB episodes
        >>> import os
        >>> os.environ['MAX_EPISODES_PER_JOB'] = '2'  # Set for test
        >>> split_episodes_into_chunks(['ep1', 'ep2', 'ep3', 'ep4', 'ep5'])
        [['ep1', 'ep2'], ['ep3', 'ep4'], ['ep5']]
        
        >>> split_episodes_into_chunks([])
        [[]]
    """
    if len(episode_ids) <= MAX_EPISODES_PER_JOB:
        return [episode_ids]
    
    chunks = []
    for i in range(0, len(episode_ids), MAX_EPISODES_PER_JOB):
        chunk = episode_ids[i:i + MAX_EPISODES_PER_JOB]
        chunks.append(chunk)
    
    return chunks

def submit_render_job(episode_chunk, user_id, job_queue_arn, job_definition_arn, chunk_index=0):
    """Submit a single render job for a chunk of episodes
    
    Args:
        episode_chunk: List of episode IDs for this job
        user_id: User ID requesting the render
        job_queue_arn: ARN of the batch job queue
        job_definition_arn: ARN of the batch job definition
        chunk_index: Index of this chunk (for multi-chunk jobs)
        
    Returns:
        Dict containing job submission result from AWS Batch
    """
    batch_client = get_batch_client()
    job_name_base = hashlib.md5(''.join(episode_chunk).encode('utf-8')).hexdigest()
    job_name = f'cut-list-render-job-{job_name_base}'
    if chunk_index > 0:
        job_name += f'-chunk-{chunk_index}'
    
    result = batch_client.submit_job(
        jobName=job_name,
        jobQueue=job_queue_arn,
        jobDefinition=job_definition_arn,
        parameters={'record_ids': ' '.join(episode_chunk), 'user_id': user_id},
    )
    
    return result

# This lambda function is triggered by an API Gateway v2 HTTP API endpoint
def handler(event, context):
    job_queue_arn = os.environ['RENDER_JOB_QUEUE']
    job_definition_arn = os.environ['RENDER_JOB_DEFINITION']

    request_body = json.loads(event['body'])
    try:
        claims = event['requestContext']['authorizer']['jwt']['claims']
        user_id = claims['sub']
    except (KeyError, TypeError):
        return {
            'statusCode': 401,
            'body': 'Unauthorized',
        }
    episode_ids = request_body['episodeIds']

    # Split episodes into manageable chunks
    episode_chunks = split_episodes_into_chunks(episode_ids)
    
    submitted_jobs = []
    for i, chunk in enumerate(episode_chunks):
        try:
            result = submit_render_job(chunk, user_id, job_queue_arn, job_definition_arn, i)
            submitted_jobs.append({
                'jobId': result['jobId'],
                'episodeIds': chunk,
                'chunkIndex': i
            })
        except Exception as e:
            # If any job submission fails, return error
            return {
                'statusCode': 500,
                'body': json.dumps({
                    'error': f'Failed to submit job for chunk {i}: {str(e)}',
                    'submittedJobs': submitted_jobs
                })
            }

    response = {
        'message': f'Successfully submitted {len(submitted_jobs)} render job(s)',
        'totalEpisodes': len(episode_ids),
        'jobChunks': len(submitted_jobs),
        'jobs': submitted_jobs
    }

    return {'statusCode': 200, 'body': json.dumps(response)}