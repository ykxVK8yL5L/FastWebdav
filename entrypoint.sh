#!/bin/bash

# Start the webdav 移除workdir 有必要再添加回来
#/usr/bin/fast-webdav --workdir='/etc/fast-webdav' &
/usr/bin/fast-webdav &
# Start the fastAPI process
uvicorn main:app --host '0.0.0.0' --port 8000 --reload --reload-include '*.ini' &
  
# Wait for any process to exit
wait -n
  
# Exit with status of process that exited first
exit $?
