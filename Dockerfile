FROM alpine:latest
ARG TARGETARCH
ARG TARGETVARIANT
RUN apk --no-cache add ca-certificates tini
RUN apk add tzdata && \
	cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
	echo "Asia/Shanghai" > /etc/timezone && \
	apk del tzdata

RUN mkdir -p /etc/fast-webdav
WORKDIR /root/
ADD fast-webdav-$TARGETARCH$TARGETVARIANT /usr/bin/fast-webdav

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/bin/fast-webdav", "--workdir", "/etc/fast-webdav"]
