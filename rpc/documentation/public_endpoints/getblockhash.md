# Get Block Hash
Returns the block hash for the given block height, if it exists in the canonical chain.

### Arguments

|   Parameter    |  Type  | Required |                 Description                  |
|:--------------:|:------:|:--------:|:--------------------------------------------:|
| `block_height` | number |   Yes    | The block height of the requested block hash |

### Response

| Parameter |  Type  |                      Description                      |
|:---------:|:------:|:-----------------------------------------------------:|
| `result`  | string | The block hash of the block at the given block height |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "getblockhash", "params": [0] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
    "jsonrpc": "2.0",
    "result": "ab18946qsq2ppqylhk03ftpg7wjuknp4gwpqz0hhp8hl2ahn94sg5zqxd8qw8",
    "id": "1"
}
```