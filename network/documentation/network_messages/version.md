A handshake request to esablish a connection with a potential peer.

### Payload

|      Parameter     | Type   |                  Description                 |
|:------------------:|--------|:--------------------------------------------:|
| `version`          | number | The serialized bytes of a transaction        |
| `height`           | number | Latest block height of the node              |
| `nonce`            | number | Random nonce to identify the version message |
| `timestamp`        | number | Message timestamp                            |
| `address_receiver` | string | IP of the message receiver                   |
| `address_sender`   | string | IP of the message sender                     |
