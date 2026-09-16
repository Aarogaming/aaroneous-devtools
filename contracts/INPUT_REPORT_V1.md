# Foundation Input Report v1

**Artifact kind:** `foundation.input-report`  
**Schema version:** `1`

This report proves which explicit inputs a foundation command accepted. It is intentionally small
and contains no ambient machine discovery, credentials, or model contents.

| Field | Meaning |
| --- | --- |
| `schema_version` | Contract version. Consumers reject versions they do not support. |
| `artifact_kind` | Stable artifact discriminator. |
| `checkout` | Absolute caller-supplied Aaroneous checkout path. |
| `output` | Absolute caller-supplied report path. |
| `support_directories` | Absolute caller-supplied support paths, in argument order. |

A future incompatible change requires a new schema version and fixture. Additive optional fields
may remain within v1 only when old consumers can safely ignore them.
