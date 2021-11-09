# Latest Ledger Root
Returns the latest ledger root from the canonical chain.

### Arguments

None

### Response

| Parameter |  Type  |              Description                |
|:---------:|:------:|:---------------------------------------:|
| `result`  | string | The ledger root of the canonical chain. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestledgerroot", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response 
```json
{
   "jsonrpc":"2.0",
   "result":"al1a0970pr25xy9gmh2qak2kajxmnwp5zvkwuk9kcjx3vneatwaxygs7trezp",
   "id":"1"
}
```