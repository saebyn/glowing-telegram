"""Shared utility functions for WebSocket Lambda handlers"""

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
