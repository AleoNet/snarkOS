Adds the given addresses to the node's list of peers and attempts to connect to them.

### Protected Endpoint

Yes

### Arguments

|      Parameter      |  Type  | Required |                    Description                   |
|:-------------------:|:------:|:--------:|:------------------------------------------------ |
| `addresses`         | array  |    Yes   | The addresses to connect to in an IP:port format |

### Response

null

### Example
```ignore
curl --user username:password --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "connect", "params": ["127.0.0.1:4141", "127.0.0.1:4142"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```
