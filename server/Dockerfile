# Build this image from the extracted Linux release archive.
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg python3.10 python3.10-venv python3-pip libsndfile1 ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN useradd --create-home --uid 1000 homer && mkdir -p /data && chown homer:homer /data
WORKDIR /opt/homer
COPY --chown=homer:homer homer-server run.sh ./
COPY --chown=homer:homer web ./web
COPY --chown=homer:homer resources ./resources
USER homer
ENV HOMER_BIND=0.0.0.0:8080 HOMER_DATA_DIR=/data
VOLUME ["/data"]
EXPOSE 8080
ENTRYPOINT ["./run.sh"]
