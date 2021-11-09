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
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockheight", "params": ["ab1mkgcrwag6avnp0a0ntxr9y4lt7y2fq9t0tqglkkuysk6jg2rhqrqa9788l"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
    "jsonrpc": "2.0",
    "result": 6160,
    "id": "1"
}
```