Returns the block given the block height.

### Arguments

|    Parameter   |  Type  | Required |                Description              |
|:--------------:|:------:|:--------:|:---------------------------------------:|
| `block_height` | number |    Yes   | The block height of the requested block |

### Response

|        Parameter            |  Type  |                            Description                            |
|:---------------------------:|:------:|:-----------------------------------------------------------------:|
| `block_hash`                | string | The hash of the block                                             |
| `previous_block_hash`       | string | The hash of the previous block                                    |
| `header`                    | number | The block header containing the state of the ledger at this block |
| `transactions`              | number | The list of transactions included in the block                    |


### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblock", "params": [100] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
