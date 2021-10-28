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
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "ledgerproof", "params": ["caf49293d36f0215cfb3296dbc871a0ef5e5dcfc61f91cd0c9ac2c730f84d853"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
