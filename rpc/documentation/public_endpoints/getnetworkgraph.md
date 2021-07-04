Returns the network graph crawled by this node (if it is a bootnode).

### Arguments

None

### Response

| Parameter                 | Type       | Description                               |
| :-----------------------: | :--------: | :---------------------------------------: |
| `edges`                   | array      | The list of connections known by the node |
| `vertices`                | array      | The list of nodes known by the node       |
| `edges[i].source`         | SocketAddr | One side of the crawled connection        |
| `edges[i].target`         | SocketAddr | The other side of the crawled connection  |
| `vertices[i].addr`        | SocketAddr | The recorded address of the crawled node  |
| `vertices[i].is_bootnode` | bool       | Indicates whether the node is a bootnode  |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnetworkgraph", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

