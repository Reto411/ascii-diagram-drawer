#!/bin/sh
# Entrypoint for the isolated pi devcontainer (see README.md, "Egress isolation").
#
# With ISOLATED_EGRESS=1 the container's network namespace is locked down at
# the firewall level (requires --cap-add NET_ADMIN): the only reachable
# destinations are
#   - 10.0.2.2:3128  egress proxy (Squid, host loopback via slirp alias)
#   - 10.0.2.2:8888  local model server (host loopback via slirp alias)
# Everything else is REJECTed. Fails closed: if the rules cannot be applied,
# the container does not start — never run "isolated" without the firewall.
#
# Args after the image name are passed to pi (same as before this wrapper).
# For non-pi commands, override explicitly: --entrypoint sh / --entrypoint bash ...
if [ "${ISOLATED_EGRESS:-0}" = "1" ]; then
  set -e
  iptables -A OUTPUT -d 127.0.0.0/8 -j ACCEPT
  iptables -A OUTPUT -d 10.0.2.2/32 -p tcp --dport 3128 -j ACCEPT
  iptables -A OUTPUT -d 10.0.2.2/32 -p tcp --dport 8888 -j ACCEPT
  iptables -A OUTPUT -j REJECT
  ip6tables -A OUTPUT -d ::1/128 -j ACCEPT
  ip6tables -A OUTPUT -j REJECT
fi
exec pi "$@"
