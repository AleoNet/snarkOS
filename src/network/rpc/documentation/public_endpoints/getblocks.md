Returns up to `MAX_RESPONSE_BLOCKS` blocks from the given `start_block_height` to `end_block_height`.
### Arguments

|       Parameter      |  Type  | Required |                         Description                         |
|:--------------------:|:------:|:--------:|:-----------------------------------------------------------:|
| `start_block_height` | number |    Yes   | The block height of the first requested block in the array. |
| `end_block_height`   | number |    Yes   | The block height of the last requested block in the array.  |

### Response

|     Parameter         |  Type  |                Description               |
|:---------------------:|:------:|:----------------------------------------:|
| `None`                |  array | The array of requested blocks            |


#### JSON Block object
Each block in the array will contain the following:

|        Parameter            |  Type  |                            Description                            |
|:---------------------------:|:------:|:-----------------------------------------------------------------:|
| `block_hash`                | string | The hash of the block                                             |
| `previous_block_hash`       | string | The hash of the previous block                                    |
| `header`                    | number | The block header containing the state of the ledger at this block |
| `transactions`              | number | The list of transactions included in the block                    |


### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblocks", "params": [1, 100] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
