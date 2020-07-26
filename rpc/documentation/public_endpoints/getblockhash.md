### getblockhash

Returns the block hash of a block at the given block height in the best valid chain.

#### Arguments

|    Parameter   |  Type  | Required |                  Description                 |
|:-------------- |:------:|:--------:|:-------------------------------------------- |
| `block_height` | string |    Yes   | The block height of the requested block hash |

#### Response

| Parameter |  Type  |                      Description                      |
|:---------:|:------:|:-----------------------------------------------------:|
| `result`  | string | The block hash of the block at the given block height |

#### Example
```
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockhash", "params": [100] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
