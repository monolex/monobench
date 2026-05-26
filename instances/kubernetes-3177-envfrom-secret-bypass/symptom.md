On a Kubernetes cluster using the ServiceAccount "mountable secrets" restriction
(`kubernetes.io/enforce-mountable-secrets`), a Pod that references a Secret **not** in the service
account's allowed list is still admitted — when the Secret is consumed through one particular
container field. Pods that reference the same disallowed Secret through volumes, individual
environment variables, or image-pull secrets are correctly rejected.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
