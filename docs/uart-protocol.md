# UART protocol summary

The UART transport and frame parser are provided by `rubo_engine`. See
`doc/uart_control_protocol.md` for the complete electrical, framing, response,
and troubleshooting specification.

## Port settings

- Ubuntu development: `/tmp/rubo-uart`, `115200`
- Orange Pi: `/dev/ttyAMA1`, `9600`
- Raspberry Pi: `/dev/serial0`, `9600`
- Data bits: `8`
- Stop bits: `1`
- Parity: none

## Request frame

Every request is four bytes:

```text
0x61 COMMAND 0x0D 0x0A
```

## Commands

| Binding key | COMMAND byte | Function |
| --- | --- | --- |
| `"1"` | `0x01` | Color detection |
| `"2"` | `0x02` | QR detection |
| `"3"` | `0x03` | Concentric-ring positioning |
| `"4"` | `0x04` | Black-ring detection |
| `"5"` | `0x05` | Letter detection |
| `"6"` | `0x06` | Colored-column positioning |
| `"17"` | `0x11` | Static color result sample (`blue`) |
| `"18"` | `0x12` | Static QR result sample (`RUBO-QR-SAMPLE`) |
| `"19"` | `0x13` | Static concentric-ring result sample |
| `"20"` | `0x14` | Static black-ring result sample |
| `"21"` | `0x15` | Static letter result sample (`A`) |
| `"22"` | `0x16` | Static colored-column result sample |
| `"49"` | `0x31` (`'1'`) | Debug |

Visual commands use raw binary bytes `0x01` through `0x06`, not ASCII digits.
The debug command is the separate ASCII byte `0x31`. Commands `0x11` through
`0x16` do not access a camera; they return fixed results to both Web and UART
sinks so the control system can test its parsers.

Colored-column positioning returns `BLOCK,<color>,<found>,<dx>,<dy>\n`.
Positive `dx` means right of the configured target point; positive `dy` means
below it. The current color names are `red`, `blue`, `green`, `black`, and
`white`; a miss returns `BLOCK,unknown,0,0,0`.
