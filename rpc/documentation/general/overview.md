
In Aleo, every full node includes a [JSON-RPC](https://www.jsonrpc.org/specification) server
to enable API calls for fetching data and interacting with peers connected to the network.

## RPC Port

```
-rpc-port 3030
```

The default RPC port is 3030. This can be specified with the `-rpc-port` flag when starting a full node.

## Authentication

The RPC server exposes protected RPC endpoints for account specific operations, such as creating an account,
creating a transaction, and fetching record commitments.
RPC requests to protected RPC endpoints can be optionally guarded with an authentication header.

To enable this authentication layer, provide the authentication credentials to
the `-rpc-username` and `-rpc-password` flags when booting up a full node.
