# Description: Run the test_api container

# Note: The openai_key.txt file is mounted to the container so that the API key is not hard-coded in the container
# - The .env file is used to pass the environment variables to the container
# - The --rm flag is used to remove the container after it exits
# - The -it flag is used to run the container in interactive mode
# - The -p flag is used to map the host port 3000 to the container port 3000
# - The --name flag is used to name the container
# - The -v flag is used to mount the openai_key.txt file to the container
# - The test_api argument is the name of the image to run
#
# multiple line commands in powershell use the backtick (`) to escape the newline
docker run --env-file .env `
           -e HOST=0.0.0.0 `
           -e OPENAI_KEY_PATH=/app/openai_key.txt `
           -p 3000:3000 `
           -it `
           --rm `
           --name test_api `
           -v "$(pwd)/../openai_key.txt:/app/openai_key.txt" `
           test_api

