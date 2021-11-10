# Latest Block Hash
Returns the block hash from the head of the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |                    Description                     |
|:---------:|:------:|:--------------------------------------------------:|
| `result`  | string | The block hash of the head of the canonical chain. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblockhash", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
   "jsonrpc":"2.0",
   "result":"ab1j0nfxgng7lw746aakxlr5k2zpvca09jsf8vs5z0u85zhy6la05yq030ttd",
   "id":"1"
}
```