# Get Block Height
Returns the block height for the given the block hash.

### Arguments

|   Parameter  |  Type  | Required |                  Description                 |
|:------------:|:------:|:--------:|:--------------------------------------------:|
| `block_hash` | string |    Yes   | The block hash of the requested block height |

### Response

| Parameter |  Type  |                       Description                     |
|:---------:|:------:|:-----------------------------------------------------:|
| `result`  | number | The block height of the block at the given block hash |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockheight", "params": ["ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
  "jsonrpc": "2.0",
  "result": 0,
  "id": "1"
}
```