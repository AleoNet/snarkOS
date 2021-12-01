# Get Block Hashes
Returns up to `MAXIMUM_BLOCK_REQUEST` block hashes from the given `start_block_height` to `end_block_height` (inclusive).

### Arguments

|       Parameter      |  Type  | Required |                      Description                     |
|:--------------------:|:------:|:--------:|:----------------------------------------------------:|
| `start_block_height` | number |    Yes   | The start block height of the requested block hashes |
| `end_block_height`   | number |    Yes   | The end block height of the requested block hashes   |

### Response

| Parameter |  Type  |                    Description                   |
|:---------:|:------:|:------------------------------------------------:|
| `result`  | array  | The list of block hashes of the requested blocks |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockhashes", "params": [0, 1] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
  "jsonrpc": "2.0",
  "result": [
    "ab18946qsq2ppqylhk03ftpg7wjuknp4gwpqz0hhp8hl2ahn94sg5zqxd8qw8",
    "ab1a04ehlymquvlsuht7ssyh59p68z9249fla2dpque8rzke6s7gyqshxg4dn"
  ],
  "id": "1"
}
```