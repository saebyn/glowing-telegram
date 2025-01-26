FROM public.ecr.aws/lambda/python:3

# Copy function code
COPY media_lambda/main.py ${LAMBDA_TASK_ROOT}

# Set the CMD to your handler (could also be done as a parameter override outside of the Dockerfile)
CMD [ "main.handler" ]