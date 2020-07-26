### getblocktemplate

Returns the current mempool and consensus information known by this node.

#### Arguments

None

#### Response

|       Parameter       |  Type  |                      Description                      |
|:--------------------- |:------:|:----------------------------------------------------- |
| `previous_block_hash` | string | The hash of current highest block                     |
| `block_height`        | number | The height of the next block                          |
| `time`                | number | The current timestamp                                 |
| `difficulty_target`   | number | The block difficulty target                           |
| `transactions`        | array  | The list of raw transactions to include in the block  |
| `coinbase_value`      | number | The amount spendable by the coinbase transaction      |

#### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblocktemplate", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
