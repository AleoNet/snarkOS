Returns the block height for the given the block hash.

### Arguments

|   Parameter  |  Type  | Required |                  Description                 |
|:------------:|:------:|:--------:|:--------------------------------------------:|
| `block_hash` | number |    Yes   | The block hash of the requested block height |

### Response

| Parameter |  Type  |                       Description                     |
|:---------:|:------:|:-----------------------------------------------------:|
| `result`  | string | The block height of the block at the given block hash |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockheight", "params": ["caf49293d36f0215cfb3296dbc871a0ef5e5dcfc61f91cd0c9ac2c730f84d853"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
