# Operator Roots authorize; client MCP Roots do not

MCP clients can advertise Roots, but that capability is deprecated (SEP-2577) and is a confused-deputy hole: an agent could name a path the operator never allowed.

Authorization is only the Roots in the operator's config file. Jail runs on every emitted path. Tools cannot add Roots. HTTP does not start without a shared secret.

**Status**: accepted
