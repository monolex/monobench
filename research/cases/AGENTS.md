# Monobench Case Analysis Agent Guide

Each child directory is one benchmark case. Keep all cases comparable.

## Case Directory Contract

```text
<case-id>/
├── README.md
├── analysis/
└── runs/
    └── <run-id>/
        ├── README.md
        ├── SHA256SUMS
        └── raw/
```

## Case README Requirements

Include:

- case id
- upstream project/version
- symptom
- ground truth root cause
- decoys
- why the case is hard
- archived runs table
- open research questions

## Run Requirements

Every run directory must preserve:

- `raw/results/*.answer.txt`
- `raw/results/*.codexlog`
- `raw/results/*.err`
- `raw/results/*.meter.json`
- `raw/results/mcp-*.json`, if present
- `raw/instance/ground_truth.md`
- `raw/instance/instance.json`
- `raw/instance/symptom.md`
- `SHA256SUMS`

## Analysis Requirements

Every analysis document must record:

- grades from the archived monobench report
- per-run root-cause answer
- the first correct-candidate discovery point
- wrong-candidate lock-in points
- command patterns that helped or hurt
- actionable monogram improvement candidates
