Disconnects the node from the given address.

### Protected Endpoint

Yes

### Arguments

|      Parameter      |  Type  | Required |                  Description                   |
|:-------------------:|:------:|:--------:|:---------------------------------------------- |
| `address`           | string |    Yes   | The address to disconnect in an IP:port format |

### Response

null

### Example
```ignore
curl --user username:password --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "disconnect", "params": ["127.0.0.1:4141"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
