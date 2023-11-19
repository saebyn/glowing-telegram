# task_api

## Description

This is a simple task API that allows you to create, read, delete/cancel tasks. Tasks are background
processes that are requested by one of the other services in the system. The task API is responsible for
managing the state of the tasks, and for executing the tasks. The tasks are executed by making a request
to the target service. 

## API

### Create a task

```
POST /tasks
```

#### Request

```json
{
    "service": "service_name",
    "action": "action_name",
    "payload": {
        "key": "value"
    }
}
```

#### Response

Redirects to the task resource.

```http
HTTP/1.1 302 Found
Location: /tasks/1
```



#### Statuses

| Status | Description |
|--------|-------------|
| pending | The task has been created, but has not been executed yet. |
| running | The task is currently running. |
| complete | The task has completed successfully. |
| failed | The task has failed. |

#### Implementation

The task is created in the database, and the task is added to the task queue. The task queue is a
Redis list. The task queue is consumed by the task worker. The task worker is a separate service
that is responsible for executing the tasks. The task worker is a long running process that polls
the task queue for new tasks. When a new task is found, the task worker makes a request to the
service that requested the task. The task worker updates the status of the task in the database
as it progresses.

Services are registered via configuration of the task_api service.

Requests by the task worker to the service that requested the task are made via the service's
REST API. The task worker makes one or more requests to the service's REST API with the task name and the
payload of the task. The service's response status code will indicate the status of the task.

The response body of the service's response will be used as the payload of the next request to the
service. The task worker will continue to make requests to the service until the service returns
a status code that indicates that the task is complete.

| Status Code | Description |
|-------------|-------------|
| 200 | The task was successful. |
| 206 | The task is still running and another request should be made to progress the task. |
| 500 | The task failed. |



### Get a task

```
GET /tasks/:id
```

#### Response

```json
{
    "location": "/tasks/1",
    "service": "service_name",
    "action": "action_name",
    "payload": {
        "key": "value"
    },
    "status": "pending"
}
```

#### Statuses

| Status | Description |
|--------|-------------|
| pending | The task has been created, but has not been executed yet. |
| running | The task is currently running. |
| complete | The task has completed successfully. |
| failed | The task has failed. |


### Delete a task

```
DELETE /tasks/:id
```

#### Response

```http
HTTP/1.1 204 No Content
```


### List tasks

```
GET /tasks
```

#### Response

```json
[
    {
        "location": "/tasks/1"
        "service": "service_name",
        "action": "action_name",
        "payload": {
            "key": "value"
        },
        "status": "pending"
    }
]
```

#### Statuses

| Status | Description |
|--------|-------------|
| pending | The task has been created, but has not been executed yet. |
| running | The task is currently running. |
| complete | The task has completed successfully. |
| failed | The task has failed. |

