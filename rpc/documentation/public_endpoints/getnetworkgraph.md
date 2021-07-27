Returns the network graph crawled by this node (if it is a bootnode).

### Arguments

None

### Response

| Parameter                            | Type       | Description                                                                               |
| :----------------------------------: | :--------: | :---------------------------------------------------------------------------------------: |
| `node_count`                         | usize      | The number of nodes known by the node                                                     |
| `connection_count`                   | usize      | The number of connection known by the node                                                |
| `density`                            | f64        | The density of the known network                                                          |
| `algebraic_connectivity`             | f64        | The algebraic connectivity, aka the fiedler eigenvalue of the known network               |
| `degree_centrality_delta`            | f64        | The difference between the largest and the smallest peer count in the network             |
| `edges`                              | array      | The list of connections known by the node                                                 |
| `vertices`                           | array      | The list of nodes known by the node                                                       |
| `edges[i].source`                    | SocketAddr | One side of the crawled connection                                                        |
| `edges[i].target`                    | SocketAddr | The other side of the crawled connection                                                  |
| `vertices[i].addr`                   | SocketAddr | The recorded address of the crawled node                                                  |
| `vertices[i].is_bootnode`            | bool       | Indicates whether the node is a bootnode                                                  |
| `vertices[i].degree_centrality`      | u16        | The node's degree centrality, aka its connection count                                    |
| `vertices[i].eigenvector_centrality` | f64        | The node's eigenvector centrality, indicates its relative importance in the network       |
| `vertices[i].fiedler_value`          | f64        | The node's fiedler value, can be used to partition the network graph                      |

### Example
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getnetworkgraph", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

