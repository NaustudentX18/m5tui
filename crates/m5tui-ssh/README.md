# m5tui-ssh

SSH client abstraction for m5Tui.

M3 stub: `SshClient` + `Channel` traits with `StubSshClient` returning
canned output. A real russh-based impl is deferred to the network
integration phase.
