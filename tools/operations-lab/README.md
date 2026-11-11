# operations-lab

Interactive lab stand for the Gecko Operation module `0.1.0-dev` (GKO-68). Two terminal UIs in separate processes: a lab Gecko with a controllable stub executor and fault controls, and a caller that sends Start, Get and Observe over Zenoh by hand. The scenarios to run and the place to record results are in the [lab runbook](../../docs-modules/operation/lab-0.1.0.md).

It is a test tool, not part of the product. It enables the `test-support` features of [`operations`](../../crates/operations/README.md) and [`operations-zenoh`](../../crates/operations-zenoh/README.md): the permissive test policy, the proxy, and the test hooks. For that reason the plain `make build` leaves it out, so these features are not unified into the production crates.

## Run

From the repository root, on Linux or in WSL ([environment](../../docs/development-environment.md)), each command in its own terminal:

```sh
make lab-server                          # Gecko on tcp/127.0.0.1:7447, proxy on 7448
make lab-client                          # a caller connected directly
make lab-client LAB_ARGS=--via-proxy     # a caller behind the cuttable proxy
```

Options (put them in `LAB_ARGS`, or run `cargo run -p operations-lab -- server|client ...`):

| Role | Option | Default |
|---|---|---|
| server | `--port`, `--proxy-port` | `7447`, `7448` |
| server | `--lease-secs`: Observe stream lease | `20` |
| server | `--observer-capacity`: snapshots buffered per observer before `lagged` | `4` (Core's default is 64; small so lag is reachable by hand) |
| both | `--log PATH` | `target/operations-lab/<role>-<time>.log` |
| client | `--connect ENDPOINT`, `--via-proxy` | `tcp/127.0.0.1:7447`; `--via-proxy` uses `7448` |
| client | `--timeout-ms`: Start and Get reply timeout | `2000` |

Quit with `q`. Each process prints its log path on exit. The log file has every line, with a UTC time; the screen shows the last lines.

## Server screen

Settings apply to the next handoffs; executions already running keep their script.

| Key | Effect |
|---|---|
| `a` | Handoff answer: acknowledge, slow acknowledge (3 s, the execution stays `pending`), refuse (`failed`), lose the acknowledgement (`unknown`) |
| `+` `-` `i` `o` | Automatic progress: number of steps, interval (200 ms, 1 s, 3 s, 100 ms), then succeed, fail, or hold for a manual finish |
| `d` | Start reply delay: 0, 1, 3, 6 s (the reply is sent after the commit and the handoff) |
| `l` | Cut or restore the proxy link |
| `w` | Pause or resume Observe streams reading Core (observers fall behind and end as `lagged`) |
| `c` | Make the next commit of a new execution fail |
| `↑` `↓` | Select an execution |
| `p` `s` `f` `k` `r` | On the selected execution: send a progress, succeed, fail, `ack` (late evidence for `unknown`), resend the last event unchanged (a duplicate) |

The executions table shows, per execution: state and `sequence` at Core, handoffs (must stay 1), the handoff answer, executor events sent, observers attached at Core, the progress stage and the result. Core totals count execution records, retry keys, and handoffs.

## Client screen

`↑` `↓` select a field, `Enter` edits it (`Enter` keeps, `Esc` undoes, `Ctrl+U` clears), pasting replaces the selected field. An accepted Start fills `operation id`.

| Key | Effect |
|---|---|
| `s` | Start with the form; pressing it again with the same `request_id` is a retry |
| `n` | New `request_id` |
| `b` / `B` | Burst: `burst size` concurrent Starts with the same `request_id`, identical content / a different `message` each |
| `m` | Start with a payload that is not JSON |
| `g` | Get `operation id` (text that is not an id is sent as the key) |
| `o` | Observe `operation id` with the `after` field as cursor |
| `r` | Resume: a new stream after the latest `sequence` held |
| `x` | Drop the stream |

`repeat` is sent as an integer when it parses, otherwise as text (an invalid parameter). The Observe panel shows the stream state, the latest `sequence` held, and violations: a snapshot at or below the held one, or a jump without a disclosed gap, is counted and logged in red. The header shows whether the Zenoh session is connected; after a restored link, Zenoh reconnects by itself, which took about 3 s on the development host.

## Tests

`tests/lab.rs` drives the controllers (no terminal) over a real Zenoh session: lost acknowledgement refined by evidence, lost reply with retry and conflict, bursts, and Observe to the terminal with a resume. They check that the stand works; they are not the manual experiment. The screens were checked by running both processes in `tmux` on the development host.

## Layout

```text
tools/operations-lab/
├── src/main.rs        arguments, roles
├── src/server.rs      Lab: Core, Zenoh server, proxy, stub executor, faults
├── src/client.rs      LabClient: form, actions, Observe stream checks
├── src/server_ui.rs   server screen
├── src/client_ui.rs   client screen
├── src/tui.rs         terminal loop and shared widgets
├── src/logbook.rs     timestamped log in memory and in a file
└── tests/lab.rs
```
