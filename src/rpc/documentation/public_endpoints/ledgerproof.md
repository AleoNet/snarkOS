Returns the ledger root and ledger inclusion proof for a given block hash.

### Arguments

| Parameter           | Type   | Required | Description                                                        |
|---------------------|--------|----------|--------------------------------------------------------------------|
| `record_commitment` | number | Yes      | The record commitment to generate a ledger proof of inclusion for. |

### Response

| Parameter      | Type   | Description                                                 |
|----------------|--------|-------------------------------------------------------------|
| `ledger_proof` | string | The ledger inclusion proof for the given record commitment. |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "ledgerproof", "params": ["cm1f2813d13ec6ca2b97fe3d3089ae8fbe2f2813d13ec6ca2b97fe3d3089a"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
