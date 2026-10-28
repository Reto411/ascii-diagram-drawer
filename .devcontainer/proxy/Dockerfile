# Egress proxy (Squid) for the isolated pi devcontainer.
# Allow-list only — see squid.conf and .devcontainer/README.md ("Egress isolation").
FROM debian:trixie-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends squid \
 && rm -rf /var/lib/apt/lists/*

COPY squid.conf /etc/squid/squid.conf

EXPOSE 3128
# -N: run in the foreground (container entrypoint).
CMD ["squid", "-N"]
