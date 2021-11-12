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
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockhash", "params": [100] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
    "jsonrpc": "2.0",
    "result": "ab1mkgcrwag6avnp0a0ntxr9y4lt7y2fq9t0tqglkkuysk6jg2rhqrqa9788l",
    "id": 1
}
```