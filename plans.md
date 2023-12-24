# Ideas/plans:

- task api
  - [x] setting up api service with common_api_lib
  - [x] adding task service to docker compose
  - [x] mapping task service API under crud API in nginx to make it work nicely with the frontend
  - [x] update the readme per the TODO
  - [ ] setting up input/output structs for task APIs, having API do nothing
  - [ ] calling task API from postman
  - [x] set up task service to use redis
- task worker service
  - [x] figure out how to have tasks respond to events from redis
- task api
  - [ ] complete todos in task api
  - [ ] add task resource to react frontend
- silence detection api
  - [ ] implement detect endpoint to use task api
