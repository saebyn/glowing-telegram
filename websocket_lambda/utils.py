"""Shared utility functions for WebSocket Lambda handlers"""


def paginated_query(table, **kwargs):
    """Generator that yields items from a DynamoDB query, handling pagination automatically.
    
    Usage:
        for item in paginated_query(table, IndexName='user_id-index', 
                                    KeyConditionExpression='user_id = :userId',
                                    ExpressionAttributeValues={':userId': user_id}):
            # process item
    """
    response = table.query(**kwargs)
    for item in response.get('Items', []):
        yield item
    
    while 'LastEvaluatedKey' in response:
        kwargs['ExclusiveStartKey'] = response['LastEvaluatedKey']
        response = table.query(**kwargs)
        for item in response.get('Items', []):
            yield item


def paginated_scan(table, **kwargs):
    """Generator that yields items from a DynamoDB scan, handling pagination automatically.
    
    Usage:
        for item in paginated_scan(table, FilterExpression='...'):
            # process item
    """
    response = table.scan(**kwargs)
    for item in response.get('Items', []):
        yield item
    
    while 'LastEvaluatedKey' in response:
        kwargs['ExclusiveStartKey'] = response['LastEvaluatedKey']
        response = table.scan(**kwargs)
        for item in response.get('Items', []):
            yield item


def deserialize_dynamodb_value(value):
    """Deserialize a single DynamoDB attribute value"""
    if 'S' in value:
        return value['S']
    elif 'N' in value:
        return float(value['N']) if '.' in value['N'] else int(value['N'])
    elif 'BOOL' in value:
        return value['BOOL']
    elif 'M' in value:
        return deserialize_dynamodb_item(value['M'])
    elif 'L' in value:
        return [deserialize_dynamodb_value(item) for item in value['L']]
    elif 'SS' in value:
        return set(value['SS'])
    elif 'NS' in value:
        return set(float(n) if '.' in n else int(n) for n in value['NS'])
    elif 'BS' in value:
        return set(value['BS'])
    elif 'NULL' in value:
        return None
    else:
        return value


def deserialize_dynamodb_item(item):
    """Deserialize a complete DynamoDB item"""
    if not item:
        return None
    
    # Convert DynamoDB types to Python types
    result = {}
    for key, value in item.items():
        result[key] = deserialize_dynamodb_value(value)
    
    return result
