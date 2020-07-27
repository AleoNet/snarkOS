Returns information about a block from a block hash.

### Arguments

|  Parameter   |  Type  | Required |              Description              |
|:------------ |:------:|:--------:|:------------------------------------- |
| `block_hash` | string |    Yes   | The block hash of the requested block |

### Response

|      Parameter      |  Type  |                    Description                    |
|:------------------- |:------:|:------------------------------------------------- |
| `block_hash`        | string | The number of blocks in the best valid chain      |
| `difficulty_target` | number | The difficulty of the block                       |
| `hash`              | string | The block hash (same as provided)                 |
| `height`            | number | The block height                                  |
| `merkle_root`       | number | The merkle root of the transactions in the block  |
| `nonce`             | number | The nonce for solving the PoSW puzzle             |
| `proof`             | string | The Proof of Succinct Work                        |
| `size`              | number | The size of the block in bytes                    |
| `time`              | number | The block time                                    |
| `transactions`      | array  | The list of transaction ids included in the block |

### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblock", "params": ["caf49293d36f0215cfb3296dbc871a0ef5e5dcfc61f91cd0c9ac2c730f84d853"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
