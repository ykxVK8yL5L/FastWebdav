FROM python:3.10.10-alpine
ARG TARGETARCH
ARG TARGETVARIANT
RUN apk --no-cache add ca-certificates tini
RUN apk add tzdata && \
	cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
	echo "Asia/Shanghai" > /etc/timezone && \
	apk del tzdata

COPY ./FastAPI /root
VOLUME /root/configs/
COPY entrypoint.sh /entrypoint.sh
RUN apk add --no-cache bash && chmod +x /entrypoint.sh
RUN mkdir -p /etc/fast-webdav
WORKDIR /root/
RUN pip install --no-cache-dir -r requirements.txt

ADD fast-webdav-$TARGETARCH$TARGETVARIANT /usr/bin/fast-webdav
#CMD ["/usr/bin/fast-webdav", "--workdir", "/etc/fast-webdav"]
CMD [ "/entrypoint.sh" ]
