# m5tui-persist

Persistence layer for m5Tui.

M3 stub: `MemoryDriver` and `FileDriver` implement the `PersistDriver`
trait. Theme YAML load/save and scrollback bytes are supported. Real
SD-card atomic writes and schema migration are deferred to the hardware
phase (M3.x or device integration).
