ARG PYTHON_VER=3.10
ARG PYTHON_IMG_TYPE=alpine

FROM python:3.10-alpine3.18 AS builder

ARG PYTHON_VER=3.10
ARG PYTHON_IMG_TYPE=alpine
ARG EXT_TYPE=essential
RUN if test "${PYTHON_IMG_TYPE}" = 'alpine' && test "${EXT_TYPE}" != 'essential'; then \
    apk add --update gcc libxml2-dev libxslt-dev musl-dev make cargo; \
    elif test "${PYTHON_IMG_TYPE}" = 'slim' && test "${EXT_TYPE}" != 'essential'; then \
    apt-get update -qq; apt-get install  --no-install-recommends libc6-dev gcc libxml2-dev libxslt-dev make cargo -y;  \
    fi
RUN apk add --update gcc libxml2-dev libxslt-dev musl-dev make cargo
COPY FastAPI/requirements-${EXT_TYPE}.txt /tmp/requirements.txt
RUN pip wheel -r /tmp/requirements.txt --wheel-dir /tmp/wheels

FROM python:3.10-alpine3.18
COPY --from=builder /tmp/wheels/* /tmp/wheels/
RUN pip install /tmp/wheels/*.whl && rm -rf /tmp
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

ADD fast-webdav-$TARGETARCH$TARGETVARIANT /usr/bin/fast-webdav
#CMD ["/usr/bin/fast-webdav", "--workdir", "/root/"]
CMD [ "/entrypoint.sh" ]
