Returns the ledger root and ledger inclusion proof for a given block hash.

### Arguments

|   Parameter  |  Type  | Required |                         Description                         |
|:------------:|:------:|:--------:|:-----------------------------------------------------------:|
| `block_hash` | number |    Yes   | The block hash to generate a ledger proof of inclusion for. |

### Response

|                Parameter                |       Type       |                                      Description                                     |
|:---------------------------------------:|:----------------:|:------------------------------------------------------------------------------------:|
| `(ledger_root, ledger_inclusion_proof)` | (string, string) | The root of the ledger tree and the ledger inclusion proof for the given block hash. |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "ledgerproof", "params": ["ab1zrtj5r0mhn575fufayacc8klqf7u4pqfnyh82e3uacapz0e9qgqszcu2dt"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
