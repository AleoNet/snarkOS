# Get Blocks Mined
Returns 

### Arguments

None

### Response

|        Parameter            |  Type  | Description |
|:---------------------------:|:------:|:-----------:|
| `canon_blocks_mined`        | number | The amount of canon records corresponding to all addresses found on disk. |
| `total_blocks_mined`        | number | The amount of all records corresponding to all addresses found on disk. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblocksmined", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
   "jsonrpc":"2.0",
   "result":{
      "canon_blocks_mined": 3,
      "total_blocks_mined": 7,   
   },
   "id":"1"
}
```
