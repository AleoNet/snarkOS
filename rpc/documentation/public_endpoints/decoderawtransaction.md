Returns information about a transaction from serialized transaction bytes.

### Arguments

|      Parameter      |  Type  | Required |            Description            |
|:------------------- |:------:|:--------:|:--------------------------------- |
| `transaction_bytes` | string |    Yes   | The raw transaction hex to decode |

### Response

|        Parameter        |  Type  |                Description                |
|:-----------------------:|:------:|:----------------------------------------- |
| `txid`                  | string | The transaction id                        |
| `size`                  | number | The size of the transaction in bytes      |
| `old_serial_numbers`    | array  | The list of old record serial numbers     |
| `new_commitments`       | array  | The list of new record commitments        |
| `memo`                  | string | The transaction memo                      |
| `digest`                | string | The merkle tree digest                    |
| `transaction_proof`     | string | The transaction zero knowledge proof      |
| `predicate_commitment`  | string | The predicate verification key commitment |
| `local_data_commitment` | string | The local data commitment                 |
| `value balance`         | number | The transaction value balance             |
| `signatures`            | array  | The list of transaction signatures        |
| `transaction_metadata`  | object | The transaction metadata                  |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "decoderawtransaction", "params": ["transaction_hexstring"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
