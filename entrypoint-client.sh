#!/bin/bash


# export FASTWEBDAV_HEADERS="Authorization: Bearer token, Content-Type: application/json"
# 组合自定义headers
header_string=""
if [ -n "$FASTWEBDAV_HEADERS" ]; then
    IFS=',' read -ra headers <<< "$FASTWEBDAV_HEADERS"
    for header in "${headers[@]}"; do
        header_string="$header_string -h \"$header\""
    done
fi

# 配置文件路径为/root/configs/
#/usr/bin/fast-webdav --workdir='/etc/fast-webdav' &
/usr/bin/fast-webdav --workdir='/root/' $header_string &

# Wait for any process to exit
wait -n
  
# Exit with status of process that exited first
exit $?
