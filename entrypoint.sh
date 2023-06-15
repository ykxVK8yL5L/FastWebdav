#!/bin/bash

# Start the webdav
/usr/bin/fast-webdav --workdir='/etc/fast-webdav' &

# Start the fastAPI process
 uvicorn main:app --reload --reload-include '*.ini' &
  
# Wait for any process to exit
wait -n
  
# Exit with status of process that exited first
exit $?
