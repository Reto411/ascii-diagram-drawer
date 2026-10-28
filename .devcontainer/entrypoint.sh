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

# Seed a fresh agent-home volume with baked-in pi packages: the volume mount
# shadows /root/.pi/agent, so copy them from /opt/pi-seed (installed at build
# time) when missing. Never overwrites existing state. To force a re-seed of
# a broken install: delete ~/.pi/agent/git and/or ~/.pi/agent/npm in the volume.
for d in git npm; do
  if [ -d "/opt/pi-seed/$d" ] && [ ! -e "$HOME/.pi/agent/$d" ]; then
    cp -a "/opt/pi-seed/$d" "$HOME/.pi/agent/$d"
  fi
done

if [ "${ISOLATED_EGRESS:-0}" = "1" ]; then
  set -e
  iptables -A OUTPUT -d 127.0.0.0/8 -j ACCEPT
  iptables -A OUTPUT -d 10.0.2.2/32 -p tcp --dport 3128 -j ACCEPT
  iptables -A OUTPUT -d 10.0.2.2/32 -p tcp --dport 8888 -j ACCEPT
  iptables -A OUTPUT -j REJECT
  ip6tables -A OUTPUT -d ::1/128 -j ACCEPT
  ip6tables -A OUTPUT -j REJECT
fi

# Drop CAP_NET_ADMIN now that the rules are in place: without it, even root
# in this container cannot modify netfilter (verified: iptables ops fail with
# EPERM), so the agent cannot remove its own isolation. capsh re-execs pi
# with the reduced capability set; the rules themselves live in the kernel's
# netns and stay in effect.
if [ "${ISOLATED_EGRESS:-0}" = "1" ]; then
  exec capsh --drop=cap_net_admin --shell=/bin/sh -- -c 'exec pi "$@"' pi "$@"
fi
exec pi "$@"
