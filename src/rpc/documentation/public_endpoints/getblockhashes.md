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
        "ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh",
        "ab1zfygptd2x8hacsgjeew39fnpurahvqhdj7qe4kc050rhzrdclcqsgng7rk"
    ],
    "id": "1"
}
```