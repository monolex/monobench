# Case: ksmbd-37899

## Status

Initial monogram log archived.

## Source

Original monobench root:

`/Users/macbook/.monobench/0.1.2-1779431036`

Durable run archive:

`runs/2026-05-23_gpt-5.5_low_monogram-r1/`

## Ground Truth

Root cause:

`smb2pdu.c::smb2_session_logoff`

Mechanism:

SMB3 multichannel allows multiple connections to share one session. `smb2_session_logoff()` waited only for the current connection to become idle, then freed `sess->user` while other bound connections could still access the same session.

## Current Archived Results

| Run | Grade | Answer Root Cause |
|---|---:|---|
| `monogram-low-r1` | FULL | `smb2pdu.c::smb2_session_logoff` |

## Research Questions

- Why was this case easy for both baseline and monogram?
- Which monogram command path identified the cross-connection teardown race?
- Should this case remain in the benchmark if the baseline also solves it?
