# For each record in the dynamodb table "metadata-table-aa16405"
# create an event that looks like the s3 object creation event
# for the key that is the value of the "key" attribute in the record

import boto3
import json


def main():
    dynamodb = boto3.client("dynamodb")
    eb = boto3.client("events")
    table_name = "metadata-table-aa16405"
    response = dynamodb.scan(TableName=table_name)
    for item in response["Items"]:
        key = item["key"]["S"]
        print(key)
        event = {
            "Source": "manual-test",
            "DetailType": "Object Created",
            "Detail": json.dumps(
                {
                    "bucket": {
                        "name": "saebyn-video-archive",
                    },
                    "object": {
                        "key": key,
                    },
                }
            ),
        }
        x = eb.put_events(Entries=[event])
        print(x)


if __name__ == "__main__":
    main()
