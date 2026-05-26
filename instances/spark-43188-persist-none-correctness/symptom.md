A Spark job gets **wrong results** after calling `.persist(StorageLevel.NONE)` on a Dataset: e.g.
`Seq(1, 2).toDS().persist(StorageLevel.NONE).count()` returns the wrong count. There is no exception
— the query simply produces incorrect rows once the Dataset has been persisted at this level.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
