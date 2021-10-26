Returns the block from the head of the canonical chain.

### Arguments

None

### Response

|        Parameter            |  Type  |                            Description                            |
|:---------------------------:|:------:|:-----------------------------------------------------------------:|
| `block_hash`                | string | The hash of the block                                             |
| `previous_block_hash`       | string | The hash of the previous block                                    |
| `header`                    | number | The block header containing the state of the ledger at the block. |
| `transactions`              | number | The list of transactions included in the block                    |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblock", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
