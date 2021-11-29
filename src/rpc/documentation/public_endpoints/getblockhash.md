# Get Block Hash
Returns the block hash for the given block height, if it exists in the canonical chain.

### Arguments

|    Parameter   |  Type  | Required |                  Description                 |
|:--------------:|:------:|:--------:|:--------------------------------------------:|
| `block_height` | number |    Yes   | The block height of the requested block hash |

### Response

| Parameter |  Type  |                      Description                      |
|:---------:|:------:|:-----------------------------------------------------:|
| `result`  | string | The block hash of the block at the given block height |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockhash", "params": [0] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
    "jsonrpc": "2.0",
    "result": "ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh",
    "id": 1
}
```